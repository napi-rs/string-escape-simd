use std::arch::x86_64::{
    __m128i, __m256i, _mm256_add_epi8, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_load_si256,
    _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_or_si256, _mm256_set1_epi8,
    _mm_add_epi8, _mm_cmpeq_epi8, _mm_cmpgt_epi8, _mm_load_si128, _mm_loadu_si128,
    _mm_movemask_epi8, _mm_or_si128, _mm_prefetch, _mm_set1_epi8, _MM_HINT_T0,
};

use crate::{ESCAPE, HEX_BYTES, UU};

// Constants for control character detection using signed comparison trick
const TRANSLATION_A: i8 = i8::MAX - 31i8;
const BELOW_A: i8 = i8::MAX - (31i8 - 0i8) - 1;
const B: i8 = 34i8; // '"'
const C: i8 = 92i8; // '\\'

const M256_VECTOR_SIZE: usize = std::mem::size_of::<__m256i>();
const M128_VECTOR_SIZE: usize = std::mem::size_of::<__m128i>();
const LOOP_SIZE: usize = 4 * M256_VECTOR_SIZE; // Process 128 bytes at a time
const PREFETCH_DISTANCE: usize = 256; // Prefetch 256 bytes ahead

#[inline(always)]
fn sub(a: *const u8, b: *const u8) -> usize {
    debug_assert!(b <= a);
    (a as usize) - (b as usize)
}

#[target_feature(enable = "avx2")]
pub unsafe fn encode_str_avx2<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let bytes = s.as_bytes();
    let len = bytes.len();

    // Pre-allocate with estimated capacity
    let estimated_capacity = len + len / 2 + 2;
    let mut result = Vec::with_capacity(estimated_capacity);

    result.push(b'"');

    let start_ptr = bytes.as_ptr();
    let end_ptr = bytes[len..].as_ptr();
    let mut ptr = start_ptr;
    let mut start = 0;

    if len >= M256_VECTOR_SIZE {
        let v_translation_a = _mm256_set1_epi8(TRANSLATION_A);
        let v_below_a = _mm256_set1_epi8(BELOW_A);
        let v_b = _mm256_set1_epi8(B);
        let v_c = _mm256_set1_epi8(C);

        // Handle alignment - skip if already aligned
        const M256_VECTOR_ALIGN: usize = M256_VECTOR_SIZE - 1;
        let misalignment = start_ptr as usize & M256_VECTOR_ALIGN;
        if misalignment != 0 {
            let align = M256_VECTOR_SIZE - misalignment;
            let mut mask = {
                let a = _mm256_loadu_si256(ptr as *const __m256i);
                _mm256_movemask_epi8(_mm256_or_si256(
                    _mm256_or_si256(_mm256_cmpeq_epi8(a, v_b), _mm256_cmpeq_epi8(a, v_c)),
                    _mm256_cmpgt_epi8(_mm256_add_epi8(a, v_translation_a), v_below_a),
                ))
            };

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                while cur < align {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
            ptr = ptr.add(align);
        }

        // Main loop processing 128 bytes at a time
        if LOOP_SIZE <= len {
            while ptr <= end_ptr.sub(LOOP_SIZE) {
                debug_assert_eq!(0, (ptr as usize) % M256_VECTOR_SIZE);

                // Prefetch next iteration's data
                if ptr.add(LOOP_SIZE + PREFETCH_DISTANCE) < end_ptr {
                    _mm_prefetch(ptr.add(LOOP_SIZE + PREFETCH_DISTANCE) as *const i8, _MM_HINT_T0);
                }

                // Load all 4 vectors at once for better pipelining
                let a0 = _mm256_load_si256(ptr as *const __m256i);
                let a1 = _mm256_load_si256(ptr.add(M256_VECTOR_SIZE) as *const __m256i);
                let a2 = _mm256_load_si256(ptr.add(M256_VECTOR_SIZE * 2) as *const __m256i);
                let a3 = _mm256_load_si256(ptr.add(M256_VECTOR_SIZE * 3) as *const __m256i);

                // Check for quotes (") in all vectors
                let quote_0 = _mm256_cmpeq_epi8(a0, v_b);
                let quote_1 = _mm256_cmpeq_epi8(a1, v_b);
                let quote_2 = _mm256_cmpeq_epi8(a2, v_b);
                let quote_3 = _mm256_cmpeq_epi8(a3, v_b);

                // Check for backslash (\) in all vectors
                let slash_0 = _mm256_cmpeq_epi8(a0, v_c);
                let slash_1 = _mm256_cmpeq_epi8(a1, v_c);
                let slash_2 = _mm256_cmpeq_epi8(a2, v_c);
                let slash_3 = _mm256_cmpeq_epi8(a3, v_c);

                // Check for control characters (< 0x20) in all vectors
                let ctrl_0 = _mm256_cmpgt_epi8(_mm256_add_epi8(a0, v_translation_a), v_below_a);
                let ctrl_1 = _mm256_cmpgt_epi8(_mm256_add_epi8(a1, v_translation_a), v_below_a);
                let ctrl_2 = _mm256_cmpgt_epi8(_mm256_add_epi8(a2, v_translation_a), v_below_a);
                let ctrl_3 = _mm256_cmpgt_epi8(_mm256_add_epi8(a3, v_translation_a), v_below_a);

                // Combine all masks
                let cmp_a = _mm256_or_si256(_mm256_or_si256(quote_0, slash_0), ctrl_0);
                let cmp_b = _mm256_or_si256(_mm256_or_si256(quote_1, slash_1), ctrl_1);
                let cmp_c = _mm256_or_si256(_mm256_or_si256(quote_2, slash_2), ctrl_2);
                let cmp_d = _mm256_or_si256(_mm256_or_si256(quote_3, slash_3), ctrl_3);

                // Fast path: check if any escaping needed
                let any_escape = _mm256_or_si256(
                    _mm256_or_si256(cmp_a, cmp_b),
                    _mm256_or_si256(cmp_c, cmp_d),
                );

                if _mm256_movemask_epi8(any_escape) == 0 {
                    // No escapes needed, copy whole chunk
                    if start < sub(ptr, start_ptr) {
                        result.extend_from_slice(&bytes[start..sub(ptr, start_ptr)]);
                    }
                    result.extend_from_slice(std::slice::from_raw_parts(ptr, LOOP_SIZE));
                    start = sub(ptr, start_ptr) + LOOP_SIZE;
                } else {
                    // Get individual masks only when needed
                    let mask_a = _mm256_movemask_epi8(cmp_a);
                    let mask_b = _mm256_movemask_epi8(cmp_b);
                    let mask_c = _mm256_movemask_epi8(cmp_c);
                    let mask_d = _mm256_movemask_epi8(cmp_d);

                    // Process each 32-byte chunk that has escapes
                    process_mask_avx(ptr, start_ptr, &mut result, &mut start, bytes, mask_a, 0);
                    process_mask_avx(ptr, start_ptr, &mut result, &mut start, bytes, mask_b, M256_VECTOR_SIZE);
                    process_mask_avx(ptr, start_ptr, &mut result, &mut start, bytes, mask_c, M256_VECTOR_SIZE * 2);
                    process_mask_avx(ptr, start_ptr, &mut result, &mut start, bytes, mask_d, M256_VECTOR_SIZE * 3);
                }

                ptr = ptr.add(LOOP_SIZE);
            }
        }

        // Process remaining aligned chunks
        while ptr <= end_ptr.sub(M256_VECTOR_SIZE) {
            debug_assert_eq!(0, (ptr as usize) % M256_VECTOR_SIZE);
            let mut mask = {
                let a = _mm256_load_si256(ptr as *const __m256i);
                _mm256_movemask_epi8(_mm256_or_si256(
                    _mm256_or_si256(_mm256_cmpeq_epi8(a, v_b), _mm256_cmpeq_epi8(a, v_c)),
                    _mm256_cmpgt_epi8(_mm256_add_epi8(a, v_translation_a), v_below_a),
                ))
            };

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                loop {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
            ptr = ptr.add(M256_VECTOR_SIZE);
        }

        // Handle tail
        if ptr < end_ptr {
            let d = M256_VECTOR_SIZE - sub(end_ptr, ptr);
            let mut mask = ({
                let a = _mm256_loadu_si256(ptr.sub(d) as *const __m256i);
                _mm256_movemask_epi8(_mm256_or_si256(
                    _mm256_or_si256(_mm256_cmpeq_epi8(a, v_b), _mm256_cmpeq_epi8(a, v_c)),
                    _mm256_cmpgt_epi8(_mm256_add_epi8(a, v_translation_a), v_below_a),
                ))
            } as u32)
                .wrapping_shr(d as u32);

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                loop {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
        }
    } else {
        // Fall back to SSE2 for small strings
        return encode_str_sse2(input);
    }

    // Copy any remaining bytes
    if start < len {
        result.extend_from_slice(&bytes[start..]);
    }

    result.push(b'"');
    unsafe { String::from_utf8_unchecked(result) }
}

#[target_feature(enable = "sse2")]
pub unsafe fn encode_str_sse2<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let bytes = s.as_bytes();
    let len = bytes.len();

    let estimated_capacity = len + len / 2 + 2;
    let mut result = Vec::with_capacity(estimated_capacity);

    result.push(b'"');

    let start_ptr = bytes.as_ptr();
    let end_ptr = bytes[len..].as_ptr();
    let mut ptr = start_ptr;
    let mut start = 0;

    const M128_VECTOR_ALIGN: usize = M128_VECTOR_SIZE - 1;

    if len < M128_VECTOR_SIZE {
        // Scalar fallback for very small strings
        while ptr < end_ptr {
            let c = *ptr;
            let escape_byte = ESCAPE[c as usize];
            if escape_byte != 0 {
                let i = sub(ptr, start_ptr);
                if start < i {
                    result.extend_from_slice(&bytes[start..i]);
                }
                write_escape(&mut result, escape_byte, c);
                start = i + 1;
            }
            ptr = ptr.offset(1);
        }
    } else {
        let v_translation_a = _mm_set1_epi8(TRANSLATION_A);
        let v_below_a = _mm_set1_epi8(BELOW_A);
        let v_b = _mm_set1_epi8(B);
        let v_c = _mm_set1_epi8(C);

        // Handle alignment - skip if already aligned
        let misalignment = start_ptr as usize & M128_VECTOR_ALIGN;
        if misalignment != 0 {
            let align = M128_VECTOR_SIZE - misalignment;
            let mut mask = {
                let a = _mm_loadu_si128(ptr as *const __m128i);
                _mm_movemask_epi8(_mm_or_si128(
                    _mm_or_si128(_mm_cmpeq_epi8(a, v_b), _mm_cmpeq_epi8(a, v_c)),
                    _mm_cmpgt_epi8(_mm_add_epi8(a, v_translation_a), v_below_a),
                ))
            };

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                while cur < align {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
            ptr = ptr.add(align);
        }

        // Main loop
        while ptr <= end_ptr.sub(M128_VECTOR_SIZE) {
            debug_assert_eq!(0, (ptr as usize) % M128_VECTOR_SIZE);
            let mut mask = {
                let a = _mm_load_si128(ptr as *const __m128i);
                _mm_movemask_epi8(_mm_or_si128(
                    _mm_or_si128(_mm_cmpeq_epi8(a, v_b), _mm_cmpeq_epi8(a, v_c)),
                    _mm_cmpgt_epi8(_mm_add_epi8(a, v_translation_a), v_below_a),
                ))
            };

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                loop {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
            ptr = ptr.add(M128_VECTOR_SIZE);
        }

        // Handle tail
        if ptr < end_ptr {
            let d = M128_VECTOR_SIZE - sub(end_ptr, ptr);
            let mut mask = ({
                let a = _mm_loadu_si128(ptr.sub(d) as *const __m128i);
                _mm_movemask_epi8(_mm_or_si128(
                    _mm_or_si128(_mm_cmpeq_epi8(a, v_b), _mm_cmpeq_epi8(a, v_c)),
                    _mm_cmpgt_epi8(_mm_add_epi8(a, v_translation_a), v_below_a),
                ))
            } as u16)
                .wrapping_shr(d as u32);

            if mask != 0 {
                let at = sub(ptr, start_ptr);
                let mut cur = mask.trailing_zeros() as usize;
                loop {
                    let c = *ptr.add(cur);
                    let escape_byte = ESCAPE[c as usize];
                    if escape_byte != 0 {
                        let i = at + cur;
                        if start < i {
                            result.extend_from_slice(&bytes[start..i]);
                        }
                        write_escape(&mut result, escape_byte, c);
                        start = i + 1;
                    }
                    mask ^= 1 << cur;
                    if mask == 0 {
                        break;
                    }
                    cur = mask.trailing_zeros() as usize;
                }
            }
        }
    }

    // Copy any remaining bytes
    if start < len {
        result.extend_from_slice(&bytes[start..]);
    }

    result.push(b'"');
    unsafe { String::from_utf8_unchecked(result) }
}

#[inline(always)]
unsafe fn process_mask_avx(
    ptr: *const u8,
    start_ptr: *const u8,
    result: &mut Vec<u8>,
    start: &mut usize,
    bytes: &[u8],
    mask: i32,
    offset: usize,
) {
    if mask == 0 {
        return;
    }

    let ptr = ptr.add(offset);
    let at = sub(ptr, start_ptr);

    // Process mask bits using bit manipulation
    let mut remaining = mask as u32;
    while remaining != 0 {
        let cur = remaining.trailing_zeros() as usize;
        let c = *ptr.add(cur);
        let escape_byte = ESCAPE[c as usize];

        if escape_byte != 0 {
            let i = at + cur;
            // Copy unescaped portion if needed
            if *start < i {
                result.extend_from_slice(&bytes[*start..i]);
            }
            // Write escape sequence
            write_escape(result, escape_byte, c);
            *start = i + 1;
        }

        // Clear the lowest set bit
        remaining &= remaining - 1;
    }
}

#[inline(always)]
fn write_escape(result: &mut Vec<u8>, escape_byte: u8, c: u8) {
    result.push(b'\\');
    if escape_byte == UU {
        // Unicode escape for control characters
        result.extend_from_slice(b"u00");
        let hex_digits = &HEX_BYTES[c as usize];
        result.push(hex_digits.0);
        result.push(hex_digits.1);
    } else {
        // Simple escape
        result.push(escape_byte);
    }
}

// Public entry point that does runtime CPU detection
#[inline]
pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    // Runtime CPU feature detection
    if is_x86_feature_detected!("avx2") {
        unsafe { encode_str_avx2(input) }
    } else if is_x86_feature_detected!("sse2") {
        unsafe { encode_str_sse2(input) }
    } else {
        // Fallback to scalar implementation
        crate::encode_str_fallback(input)
    }
}
