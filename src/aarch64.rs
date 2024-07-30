use std::arch::aarch64::{
    uint8x16_t, vaddq_u8, vaddvq_u8, vceqq_u8, vdupq_n_u8, vld1q_u8, vld1q_u8_x4, vqtbl4q_u8,
};
use std::mem::transmute;

use crate::{encode_str_inner, write_char_escape, CharEscape, ESCAPE, REVERSE_SOLIDUS};

const CHUNK_SIZE: usize = 16;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let input_str = input.as_ref();
    let mut output = Vec::with_capacity(input_str.len() + 2);
    let bytes = input_str.as_bytes();
    let len = bytes.len();
    let writer = &mut output;
    writer.push(b'"');
    // Safety: SIMD instructions
    unsafe {
        let mut start = 0;
        while start + CHUNK_SIZE < len {
            let next_chunk = start + CHUNK_SIZE;
            let current_chunk_slice = &bytes[start..next_chunk];
            let table_low = vld1q_u8_x4(ESCAPE[0..64].as_ptr());
            let table_high = vdupq_n_u8(b'\\');
            let chunk = vld1q_u8(current_chunk_slice.as_ptr());
            let low_mask = vqtbl4q_u8(table_low, chunk);
            let high_mask = vceqq_u8(table_high, chunk);
            if vaddvq_u8(low_mask) == 0 && vaddvq_u8(high_mask) == 0 {
                writer.extend_from_slice(current_chunk_slice);
                start = next_chunk;
                continue;
            }

            // Vector add the masks to get a single mask
            let escape_table_mask = vaddq_u8(low_mask, high_mask);
            let escape_table_mask_slice = transmute::<uint8x16_t, [u8; 16]>(escape_table_mask);
            for (index, value) in escape_table_mask_slice.into_iter().enumerate() {
                if value == 0 {
                    writer.push(bytes[start + index]);
                } else if value == 255 {
                    // value is in the high table mask, which means it's `\`
                    writer.extend_from_slice(REVERSE_SOLIDUS);
                } else {
                    let char_escape =
                        CharEscape::from_escape_table(value, current_chunk_slice[index]);
                    write_char_escape(writer, char_escape);
                }
            }
            start = next_chunk;
        }

        if start < len {
            encode_str_inner(&bytes[start..], writer);
        }
    }
    writer.push(b'"');
    // Safety: the bytes are valid UTF-8
    unsafe { String::from_utf8_unchecked(output) }
}
