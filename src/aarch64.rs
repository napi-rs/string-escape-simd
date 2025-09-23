use std::arch::aarch64::{
    vceqq_u8, vdupq_n_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vqtbl4q_u8, vst1q_u8,
};

use crate::generic::{ESCAPE, HEX_BYTES, UU};

const CHUNK: usize = 64;
// 128 bytes ahead
const PREFETCH_DISTANCE: usize = CHUNK * 2;
const SLASH_SENTINEL: u8 = 0xFF;

#[inline]
pub fn escape_neon(bytes: &[u8], output: &mut Vec<u8>) {
    let n = bytes.len();

    unsafe {
        let tbl = vld1q_u8_x4(ESCAPE.as_ptr());
        let slash = vdupq_n_u8(b'\\');
        let mut i = 0usize;

        // Scratch buffer reused for mask materialisation; stay uninitialised.
        #[allow(invalid_value)]
        let mut placeholder: [u8; 16] = core::mem::MaybeUninit::uninit().assume_init();

        while i + CHUNK <= n {
            let ptr = bytes.as_ptr().add(i);

            core::arch::asm!(
                "prfm pldl1keep, [{0}]",
                in(reg) ptr.add(PREFETCH_DISTANCE),
            );

            let quad = vld1q_u8_x4(ptr);

            let a = quad.0;
            let b = quad.1;
            let c = quad.2;
            let d = quad.3;

            let mask_1 = vorrq_u8(vqtbl4q_u8(tbl, a), vceqq_u8(slash, a));
            let mask_2 = vorrq_u8(vqtbl4q_u8(tbl, b), vceqq_u8(slash, b));
            let mask_3 = vorrq_u8(vqtbl4q_u8(tbl, c), vceqq_u8(slash, c));
            let mask_4 = vorrq_u8(vqtbl4q_u8(tbl, d), vceqq_u8(slash, d));

            let mask_r_1 = vmaxvq_u8(mask_1);
            let mask_r_2 = vmaxvq_u8(mask_2);
            let mask_r_3 = vmaxvq_u8(mask_3);
            let mask_r_4 = vmaxvq_u8(mask_4);

            if mask_r_1 | mask_r_2 | mask_r_3 | mask_r_4 == 0 {
                output.extend_from_slice(std::slice::from_raw_parts(ptr, CHUNK));
                i += CHUNK;
                continue;
            }

            macro_rules! handle {
                ($mask:expr, $mask_r:expr, $off:expr) => {
                    if $mask_r == 0 {
                        output.extend_from_slice(std::slice::from_raw_parts(ptr.add($off), 16));
                    } else {
                        vst1q_u8(placeholder.as_mut_ptr(), $mask);
                        handle_block(&bytes[i + $off..i + $off + 16], &placeholder, output);
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
            handle_tail(&bytes[i..], output);
        }
    }
}

#[inline(always)]
fn handle_tail(src: &[u8], dst: &mut Vec<u8>) {
    for &c in src {
        let escape_byte = ESCAPE[c as usize];
        if escape_byte == 0 {
            dst.push(c);
        } else {
            write_escape(dst, escape_byte, c);
        }
    }
}

#[inline(always)]
fn handle_block(src: &[u8], mask: &[u8; 16], dst: &mut Vec<u8>) {
    for (j, &m) in mask.iter().enumerate() {
        let c = src[j];
        if m == 0 {
            dst.push(c);
        } else if m == SLASH_SENTINEL {
            dst.push(b'\\');
            dst.push(b'\\');
        } else {
            write_escape(dst, m, c);
        }
    }
}

#[inline(always)]
fn write_escape(dst: &mut Vec<u8>, escape_byte: u8, c: u8) {
    dst.push(b'\\');
    if escape_byte == UU {
        dst.extend_from_slice(b"u00");
        let hex = &HEX_BYTES[c as usize];
        dst.push(hex.0);
        dst.push(hex.1);
    } else {
        dst.push(escape_byte);
    }
}
