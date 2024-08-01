use std::arch::x86_64::{
    __m128i, 
    _mm_adds_epu8,
     _mm_cmpeq_epi8, _mm_loadu_si128,  _mm_set1_epi8,
      _mm_shuffle_epi8, _mm_test_all_zeros,
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
        let zero = _mm_set1_epi8(-1);

        let mut start = 0;
        while start + CHUNK_SIZE < len {
            let next_chunk = start + CHUNK_SIZE;
            let current_chunk_slice: &[u8] = &bytes[start..next_chunk];
            let table_low = _mm_loadu_si128(ESCAPE.as_ptr() as *const __m128i);
            let table_high = _mm_set1_epi8(transmute::<u8, i8>(b'\\'));
            let chunk = _mm_loadu_si128(current_chunk_slice.as_ptr() as *const __m128i);
            let low_mask = _mm_shuffle_epi8(table_low, chunk);
            let high_mask = _mm_cmpeq_epi8(table_high,chunk);
            // check every bits of mask is zero
            if _mm_test_all_zeros(low_mask, zero) != 0 && _mm_test_all_zeros(high_mask, zero) != 0 {
                writer.extend_from_slice(current_chunk_slice);
                start = next_chunk;
                continue;
            }

            // Vector add the masks to get a single mask
            // add low_mask and high_mask to get a single mask
            let escape_table_mask = _mm_adds_epu8(low_mask, high_mask);
            let escape_table_mask_slice = transmute::<__m128i, [u8; 16]>(escape_table_mask);
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