use std::arch::{
    aarch64::{vceqq_u8, vdupq_n_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vqtbl4q_u8, vst1q_u8},
    asm, is_aarch64_feature_detected,
};

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE, REVERSE_SOLIDUS};

/// Bytes handled per *outer* iteration in the new 8.7 path.
/// (Still 64 B in the NEON fallback.)
const CHUNK: usize = 64;
/// Prefetch distance (works for both paths).
const PREFETCH_DISTANCE: usize = CHUNK * 4;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let mut out = Vec::with_capacity(s.len() + 2);
    let b = s.as_bytes();
    let n = b.len();
    out.push(b'"');

    unsafe {
        #[allow(invalid_value)]
        let mut scratch: [u8; 16] = core::mem::MaybeUninit::uninit().assume_init();

        let mut i = 0;

        /* ------------------------------------------------------------------ */
        /* === Arm v8.7 fast path: LS64 + SVE2 =============================== */

        let tbl = vld1q_u8_x4(ESCAPE.as_ptr());
        let slash = vdupq_n_u8(b'\\');

        while i + CHUNK <= n {
            let ptr = b.as_ptr().add(i);
            if is_aarch64_feature_detected!("sve2") {
                i += escape_block_sve(ptr, &mut out);
                continue;
            } else {
                asm!("prfm pldl1keep, [{0}, #{1}]",
                   in(reg) ptr, const PREFETCH_DISTANCE);

                let quad = vld1q_u8_x4(ptr);
                let a = quad.0;
                let b1 = quad.1;
                let c = quad.2;
                let d = quad.3;

                let m1 = vorrq_u8(vqtbl4q_u8(tbl, a), vceqq_u8(slash, a));
                let m2 = vorrq_u8(vqtbl4q_u8(tbl, b1), vceqq_u8(slash, b1));
                let m3 = vorrq_u8(vqtbl4q_u8(tbl, c), vceqq_u8(slash, c));
                let m4 = vorrq_u8(vqtbl4q_u8(tbl, d), vceqq_u8(slash, d));

                if vmaxvq_u8(m1) | vmaxvq_u8(m2) | vmaxvq_u8(m3) | vmaxvq_u8(m4) == 0 {
                    out.extend_from_slice(std::slice::from_raw_parts(ptr, CHUNK));
                    i += CHUNK;
                    continue;
                }

                macro_rules! handle {
                    ($m:expr,$r:expr,$off:expr) => {
                        if $r == 0 {
                            out.extend_from_slice(std::slice::from_raw_parts(ptr.add($off), 16));
                        } else {
                            vst1q_u8(scratch.as_mut_ptr(), $m);
                            handle_block(&b[i + $off..i + $off + 16], &scratch, &mut out);
                        }
                    };
                }
                handle!(m1, vmaxvq_u8(m1), 0);
                handle!(m2, vmaxvq_u8(m2), 16);
                handle!(m3, vmaxvq_u8(m3), 32);
                handle!(m4, vmaxvq_u8(m4), 48);

                i += CHUNK;
            }
        }
        /* ------------------------------------------------------------------ */

        if i < n {
            encode_str_inner(&b[i..], &mut out);
        }
    }
    out.push(b'"');
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
            write_char_escape(dst, CharEscape::from_escape_table(m, c));
        }
    }
}

#[inline(always)]
unsafe fn escape_block_sve(ptr: *const u8, dst: &mut Vec<u8>) -> usize {
    /* ------------------------------------------------------------------ */
    /* One-shot: copy ESCAPE[0..64] into z4-z7                           */
    /* Each LD1B uses an in-range offset and bumps x9 by 16 bytes.       */
    core::arch::asm!(
        "ptrue  p0.b",
        "mov    x9,  {tbl}",
        "ld1b   z4.b, p0/z, [x9]",
        "add    x9,  x9,  #16",
        "ld1b   z5.b, p0/z, [x9]",
        "add    x9,  x9,  #16",
        "ld1b   z6.b, p0/z, [x9]",
        "add    x9,  x9,  #16",
        "ld1b   z7.b, p0/z, [x9]",
        tbl = in(reg) crate::ESCAPE.as_ptr(),
        out("x9") _,
        options(readonly, nostack, preserves_flags)
    );
    /* ------------------------------------------------------------------ */

    /* 1️⃣  Single-copy 64-byte fetch into L1 */
    core::arch::asm!(
        "ld64b x0, [{src}]",
        src = in(reg) ptr,
        out("x0") _, out("x1") _, out("x2") _, out("x3") _,
        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        options(nostack)
    );

    /* 2️⃣  Build escape mask */
    let mut mask: u32;
    core::arch::asm!(
        "ptrue  p0.b",
        "ld1b   z0.b, p0/z, [{src}]",
        "tbl    z1.b, {{z4.b, z5.b, z6.b, z7.b}}, z0.b",
        "dup    z2.b, {slash}",
        "cmeq   z2.b, p0/m, z0.b, z2.b",
        "orr    z3.b, z1.b, z2.b",
        "umaxv  {mask:w}, p0, z3.b",     // scalar result → wMask
        src   = in(reg) ptr,
        slash = const b'\\',
        mask  = lateout(reg) mask,
        options(preserves_flags, nostack, readonly)
    );

    if mask == 0 {
        dst.extend_from_slice(std::slice::from_raw_parts(ptr, CHUNK));
        return CHUNK;
    }

    /* 3️⃣  Spill z3 and escape the bad bytes */
    let mut m = [0u8; CHUNK];
    core::arch::asm!("ptrue p0.b", "st1b z3.b, p0, [{buf}]",
                   buf = in(reg) m.as_mut_ptr(), options(nostack));
    for (i, &bit) in m.iter().enumerate() {
        let c = *ptr.add(i);
        if bit == 0 {
            dst.push(c);
        } else if bit == 0xFF {
            dst.extend_from_slice(crate::REVERSE_SOLIDUS);
        } else {
            crate::write_char_escape(dst, CharEscape::from_escape_table(bit, c));
        }
    }
    CHUNK
}
