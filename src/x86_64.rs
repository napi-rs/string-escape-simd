use std::arch::x86_64::*;
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
        let table_low = [
            _mm_loadu_si128(ESCAPE[0..16].as_ptr() as *const __m128i),
            _mm_loadu_si128(ESCAPE[16..32].as_ptr() as *const __m128i),
            _mm_loadu_si128(ESCAPE[32..48].as_ptr() as *const __m128i),
            _mm_loadu_si128(ESCAPE[48..64].as_ptr() as *const __m128i),
        ];
        // let ones = _mm_set1_epi8(1);

        let mut start = 0;
        while start + CHUNK_SIZE < len {
            let next_chunk = start + CHUNK_SIZE;
            let current_chunk_slice: &[u8] = &bytes[start..next_chunk];
            let table_high = _mm_set1_epi8(b'\\' as i8);
            let chunk = _mm_loadu_si128(current_chunk_slice.as_ptr() as *const __m128i);
            let low_mask = table_lookup_sse42(chunk, table_low);
            let high_mask = _mm_cmpeq_epi8(table_high,chunk);
            // check every bits of mask is zero
            if horizontal_add_u8_sse42(low_mask) == 0 && horizontal_add_u8_sse42(high_mask) == 0 {
                writer.extend_from_slice(current_chunk_slice);
                start = next_chunk;
                continue;
            }

             // check every bits of mask is zero
            //  if _mm_testz_si128(low_mask, ones) == 1 && _mm_testz_si128(high_mask, ones) == 1 {
            //     writer.extend_from_slice(current_chunk_slice);
            //     start = next_chunk;
            //     continue;
            // }

            // Vector add the masks to get a single mask
            // add low_mask and high_mask to get a single mask
            let escape_table_mask = _mm_add_epi8(low_mask, high_mask);
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


fn table_lookup_sse42(indices: __m128i, table: [__m128i; 4]) -> __m128i {
    unsafe {
        // Compute the lookup results for each 16-byte chunk
        let lookup0 = _mm_shuffle_epi8(table[0], indices);
        let lookup1 = _mm_shuffle_epi8(table[1], indices);
        let lookup2 = _mm_shuffle_epi8(table[2], indices);
        let lookup3 = _mm_shuffle_epi8(table[3], indices);

        // Calculate masks to determine which lookup result to use
        let cmp0 = _mm_cmplt_epi8(indices, _mm_set1_epi8(16));
        let cmp1 = _mm_and_si128(_mm_cmplt_epi8(indices, _mm_set1_epi8(32)), _mm_cmpgt_epi8(indices, _mm_set1_epi8(15)));
        let cmp2 = _mm_and_si128(_mm_cmplt_epi8(indices, _mm_set1_epi8(48)), _mm_cmpgt_epi8(indices, _mm_set1_epi8(31)));
        let cmp3 = _mm_cmpgt_epi8(indices, _mm_set1_epi8(47));

        // Blend the lookup results based on the masks
        let result0 = _mm_blendv_epi8(_mm_setzero_si128(), lookup0, cmp0);
        let result1 = _mm_blendv_epi8(result0, lookup1, cmp1);
        let result2 = _mm_blendv_epi8(result1, lookup2, cmp2);
        let final_result = _mm_blendv_epi8(result2, lookup3, cmp3);

        final_result
    }
}

fn horizontal_add_u8_sse42(vector: __m128i) -> u8 {
    unsafe {
        // Compute the sum of the absolute differences
        let sum = _mm_sad_epu8(vector, _mm_setzero_si128());
                
        // Extract the sums from the resulting __m128i
        let sum_array = std::mem::transmute::<__m128i, [u64; 2]>(sum);
        let total_sum = sum_array[0] + sum_array[1];

        // Cast the result to u8 (sum cannot exceed 255*16 = 4080)
        total_sum as u8
    }
}