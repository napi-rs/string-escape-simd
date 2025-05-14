use std::arch::aarch64::{
    vceqq_u8, vdupq_n_u8, vld1q_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vqtbl4q_u8, vst1q_u8,
};

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE, REVERSE_SOLIDUS};

/// Four contiguous 16-byte NEON registers (64 B) per loop.
const CHUNK: usize = 64;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let mut out = Vec::with_capacity(s.len() + 2);
    let b = s.as_bytes();
    let n = b.len();
    out.push(b'"');

    unsafe {
        let tbl = vld1q_u8_x4(ESCAPE.as_ptr()); // first 64 B of the escape table
        let slash = vdupq_n_u8(b'\\');
        let mut i = 0;

        while i + CHUNK <= n {
            let ptr = b.as_ptr().add(i);

            /* ---- L1 prefetch: one cache line ahead ---- */
            core::arch::asm!("prfm pldl1keep, [{0}, #128]", in(reg) ptr);
            /* ------------------------------------------ */

            // load 64 B (four q-regs)
            let a = vld1q_u8(ptr);
            let m1 = vqtbl4q_u8(tbl, a);
            let m2 = vceqq_u8(slash, a);

            let b2 = vld1q_u8(ptr.add(16));
            let m3 = vqtbl4q_u8(tbl, b2);
            let m4 = vceqq_u8(slash, b2);

            let c = vld1q_u8(ptr.add(32));
            let m5 = vqtbl4q_u8(tbl, c);
            let m6 = vceqq_u8(slash, c);

            let d = vld1q_u8(ptr.add(48));
            let m7 = vqtbl4q_u8(tbl, d);
            let m8 = vceqq_u8(slash, d);

            let mask_1 = vorrq_u8(m1, m2);
            let mask_2 = vorrq_u8(m3, m4);
            let mask_3 = vorrq_u8(m5, m6);
            let mask_4 = vorrq_u8(m7, m8);

            let mask_r_1 = vmaxvq_u8(mask_1);
            let mask_r_2 = vmaxvq_u8(mask_2);
            let mask_r_3 = vmaxvq_u8(mask_3);
            let mask_r_4 = vmaxvq_u8(mask_4);

            // fast path: nothing needs escaping
            if mask_r_1 | mask_r_2 | mask_r_3 | mask_r_4 == 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr, CHUNK));
                i += CHUNK;
                continue;
            }
            let mut tmp: [u8; 16] = core::mem::zeroed();

            if mask_r_1 == 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr, 16));
            } else {
                vst1q_u8(tmp.as_mut_ptr(), mask_1);
                handle_block(&b[i..i + 16], &tmp, &mut out);
            }

            if mask_r_2 == 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr.add(16), 16));
            } else {
                vst1q_u8(tmp.as_mut_ptr(), mask_2);
                handle_block(&b[i + 16..i + 32], &tmp, &mut out);
            }

            if mask_r_3 == 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr.add(32), 16));
            } else {
                vst1q_u8(tmp.as_mut_ptr(), mask_3);
                handle_block(&b[i + 32..i + 48], &tmp, &mut out);
            }

            if mask_r_4 == 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr.add(48), 16));
            } else {
                vst1q_u8(tmp.as_mut_ptr(), mask_4);
                handle_block(&b[i + 48..i + 64], &tmp, &mut out);
            }

            i += CHUNK;
        }
        if i < n {
            encode_str_inner(&b[i..], &mut out);
        }
    }
    out.push(b'"');
    // SAFETY: we only emit valid UTF-8
    unsafe { String::from_utf8_unchecked(out) }
}

#[inline(always)]
unsafe fn handle_block(src: &[u8], mask: &[u8; 16], dst: &mut Vec<u8>) {
    for (j, &m) in mask.iter().enumerate() {
        let c = src[j];
        if m == 0 {
            dst.push(c);
        } else if m == 0xFF {
            dst.extend_from_slice(REVERSE_SOLIDUS);
        } else {
            let e = CharEscape::from_escape_table(m, c);
            write_char_escape(dst, e);
        }
    }
}
