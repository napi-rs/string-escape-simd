use std::arch::aarch64::{
    vceqq_u8, vdupq_n_u8, vld1q_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vqtbl4q_u8, vst1q_u8,
};

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE, REVERSE_SOLIDUS};

/// Four contiguous 16-byte NEON registers (64 B) per loop.
const CHUNK: usize = 64;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let mut out = Vec::with_capacity(s.len() + 2);
    let bytes = s.as_bytes();
    let n = bytes.len();
    out.push(b'"');

    unsafe {
        let tbl = vld1q_u8_x4(ESCAPE.as_ptr()); // first 64 B of the escape table
        let slash = vdupq_n_u8(b'\\');
        let mut i = 0;
        let mut placeholder: [u8; 16] = core::mem::zeroed();

        while i + CHUNK <= n {
            let ptr = bytes.as_ptr().add(i);

            /* ---- L1 prefetch: CHUNK size ahead ---- */
            core::arch::asm!("prfm pldl1keep, [{0}, #64]", in(reg) ptr);
            /* ------------------------------------------ */

            // load 64 B (four q-regs)
            let a = vld1q_u8(ptr);

            let b = vld1q_u8(ptr.add(16));

            let c = vld1q_u8(ptr.add(32));

            let d = vld1q_u8(ptr.add(48));

            let mask_1 = vorrq_u8(vqtbl4q_u8(tbl, a), vceqq_u8(slash, a));
            let mask_2 = vorrq_u8(vqtbl4q_u8(tbl, b), vceqq_u8(slash, b));
            let mask_3 = vorrq_u8(vqtbl4q_u8(tbl, c), vceqq_u8(slash, c));
            let mask_4 = vorrq_u8(vqtbl4q_u8(tbl, d), vceqq_u8(slash, d));

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

            macro_rules! handle {
                ($mask:expr, $mask_r:expr, $off:expr) => {
                    if $mask_r == 0 {
                        out.extend_from_slice(std::slice::from_raw_parts(ptr.add($off), 16));
                    } else {
                        vst1q_u8(placeholder.as_mut_ptr(), $mask);
                        handle_block(&bytes[i + $off..i + $off + 16], &placeholder, &mut out);
                    }
                };
            }

            handle!(mask_1, mask_r_1, 0);
            handle!(mask_2, mask_r_2, 16);
            handle!(mask_3, mask_r_3, 32);
            handle!(mask_4, mask_r_4, 48);

            i += CHUNK;
        }
        if i < n {
            encode_str_inner(&bytes[i..], &mut out);
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
