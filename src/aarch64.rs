use std::arch::{
    aarch64::{vceqq_u8, vdupq_n_u8, vld1q_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vqtbl4q_u8},
    asm,
};

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE, REVERSE_SOLIDUS};

/// We now chew 64 bytes (one cache line) at a time.
const CHUNK_SIZE: usize = 64;

#[inline(always)]
unsafe fn ld64b_to_stack(src: *const u8, dst: *mut u8) {
    // Loads 64 bytes atomically with LD64B and immediately writes them
    // to `dst` with ST64B so the following NEON code can work from L1.
    asm!(
        // x0 – x7 must be consecutive for LD64B/ST64B; we declare them
        // explicitly and ignore the values after the store.
        "ld64b  x0, [{inptr}]",
        "st64b  x0, [{outptr}]",
        inptr   = in(reg) src,
        outptr  = in(reg) dst,
        out("x0") _, out("x1") _, out("x2") _, out("x3") _,
        out("x4") _, out("x5") _, out("x6") _, out("x7") _,
        options(nostack, preserves_flags)
    );
}

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let input_str = input.as_ref();
    let mut output = Vec::with_capacity(input_str.len() + 2);
    let bytes = input_str.as_bytes();
    let len = bytes.len();
    let writer = &mut output;
    writer.push(b'"');

    unsafe {
        let mut start = 0;
        let escape_low = vld1q_u8_x4(ESCAPE.as_ptr()); // first 64 B of table
        let escape_high = vdupq_n_u8(b'\\');

        // === LS64-accelerated main loop =====================================
        while start + CHUNK_SIZE <= len {
            // 1. Pull 64 bytes to a stack buffer via LD64B/ST64B
            let mut block = [0u8; CHUNK_SIZE];
            ld64b_to_stack(bytes.as_ptr().add(start), block.as_mut_ptr());

            // 2. Process the 64 B in four 16 B slices, **unchanged** logic
            let mut slice_idx = 0;
            while slice_idx < CHUNK_SIZE {
                let chunk_ptr = block.as_ptr().add(slice_idx);
                let chunk = vld1q_u8(chunk_ptr);
                let low_mask = vqtbl4q_u8(escape_low, chunk);
                let high_mask = vceqq_u8(escape_high, chunk);

                if vmaxvq_u8(low_mask) == 0 && vmaxvq_u8(high_mask) == 0 {
                    writer.extend_from_slice(std::slice::from_raw_parts(chunk_ptr, 16));
                    slice_idx += 16;
                    continue;
                }

                // Combine masks and fall back to scalar per-byte handling
                let escape_mask = vorrq_u8(low_mask, high_mask);
                let mask_arr: [u8; 16] = core::mem::transmute(escape_mask);

                for (i, &m) in mask_arr.iter().enumerate() {
                    let b = *chunk_ptr.add(i);
                    if m == 0 {
                        writer.push(b);
                    } else if m == 0xFF {
                        writer.extend_from_slice(REVERSE_SOLIDUS);
                    } else {
                        let ce = CharEscape::from_escape_table(m, b);
                        write_char_escape(writer, ce);
                    }
                }
                slice_idx += 16;
            }
            start += CHUNK_SIZE;
        }

        if start < len {
            encode_str_inner(&bytes[start..], writer);
        }
    }

    writer.push(b'"');
    unsafe { String::from_utf8_unchecked(output) }
}
