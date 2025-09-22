use std::arch::x86_64::{
    __m256i, _mm256_cmpeq_epi8, _mm256_cmpgt_epi8, _mm256_loadu_si256, _mm256_or_si256,
    _mm256_set1_epi8, _mm256_storeu_si256, _mm256_testz_si256,
};

use crate::{encode_str_fallback, ESCAPE, HEX_BYTES, UU};

/// Four contiguous 32-byte AVX2 registers (128 B) per loop.
const CHUNK: usize = 128;
/// Distance (in bytes) to prefetch ahead.
/// Keeping ~4 iterations (4 × CHUNK = 512 B) ahead strikes a good balance
/// between hiding memory latency and not evicting useful cache lines.
const PREFETCH_DISTANCE: usize = CHUNK * 4;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let mut out = Vec::with_capacity(s.len() + 2);
    let bytes = s.as_bytes();
    let n = bytes.len();
    out.push(b'"');

    unsafe {
        let slash = _mm256_set1_epi8(b'\\' as i8);
        let quote = _mm256_set1_epi8(b'"' as i8);
        let tab = _mm256_set1_epi8(b'\t' as i8);
        let newline = _mm256_set1_epi8(b'\n' as i8);
        let carriage = _mm256_set1_epi8(b'\r' as i8);
        let backspace = _mm256_set1_epi8(0x08i8);
        let formfeed = _mm256_set1_epi8(0x0ci8);
        let ctrl_upper_bound = _mm256_set1_epi8(0x20i8);

        let mut i = 0;

        // Re-usable scratch – *uninitialised*, so no memset in the loop.
        #[allow(invalid_value)]
        let mut placeholder: [u8; 32] = core::mem::MaybeUninit::uninit().assume_init();

        while i + CHUNK <= n {
            let ptr = bytes.as_ptr().add(i);

            // Prefetch data ahead
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                core::arch::x86_64::_mm_prefetch(
                    ptr.add(PREFETCH_DISTANCE) as *const i8,
                    core::arch::x86_64::_MM_HINT_T0,
                );
            }

            // Load 128 bytes (four 32-byte chunks)
            let a = _mm256_loadu_si256(ptr as *const __m256i);
            let b = _mm256_loadu_si256(ptr.add(32) as *const __m256i);
            let c = _mm256_loadu_si256(ptr.add(64) as *const __m256i);
            let d = _mm256_loadu_si256(ptr.add(96) as *const __m256i);

            // For each chunk, check if it needs escaping
            let mask_1 = process_chunk(
                a, slash, quote, tab, newline, carriage, backspace, formfeed, ctrl_upper_bound,
            );
            let mask_2 = process_chunk(
                b, slash, quote, tab, newline, carriage, backspace, formfeed, ctrl_upper_bound,
            );
            let mask_3 = process_chunk(
                c, slash, quote, tab, newline, carriage, backspace, formfeed, ctrl_upper_bound,
            );
            let mask_4 = process_chunk(
                d, slash, quote, tab, newline, carriage, backspace, formfeed, ctrl_upper_bound,
            );

            // Check if any chunk needs escaping
            let any_escape = _mm256_or_si256(
                _mm256_or_si256(mask_1, mask_2),
                _mm256_or_si256(mask_3, mask_4),
            );

            // Fast path: nothing needs escaping
            if _mm256_testz_si256(any_escape, any_escape) != 0 {
                out.extend_from_slice(std::slice::from_raw_parts(ptr, CHUNK));
                i += CHUNK;
                continue;
            }

            // Slow path: handle each 32-byte chunk
            macro_rules! handle {
                ($mask:expr, $off:expr) => {
                    if _mm256_testz_si256($mask, $mask) != 0 {
                        // No escapes in this chunk
                        out.extend_from_slice(std::slice::from_raw_parts(ptr.add($off), 32));
                    } else {
                        // Store mask and process byte by byte
                        _mm256_storeu_si256(placeholder.as_mut_ptr() as *mut __m256i, $mask);
                        handle_block(&bytes[i + $off..i + $off + 32], &placeholder, &mut out);
                    }
                };
            }

            handle!(mask_1, 0);
            handle!(mask_2, 32);
            handle!(mask_3, 64);
            handle!(mask_4, 96);

            i += CHUNK;
        }

        // Handle remaining bytes using the fallback
        if i < n {
            let remaining_str = std::str::from_utf8(&bytes[i..]).unwrap();
            let escaped = encode_str_fallback(remaining_str);
            // Remove the quotes that encode_str_fallback adds
            let escaped_bytes = escaped.as_bytes();
            out.extend_from_slice(&escaped_bytes[1..escaped_bytes.len() - 1]);
        }
    }
    out.push(b'"');
    // SAFETY: we only emit valid UTF-8
    unsafe { String::from_utf8_unchecked(out) }
}

#[inline(always)]
unsafe fn process_chunk(
    data: __m256i,
    slash: __m256i,
    quote: __m256i,
    tab: __m256i,
    newline: __m256i,
    carriage: __m256i,
    backspace: __m256i,
    formfeed: __m256i,
    ctrl_upper_bound: __m256i,
) -> __m256i {
    // Check for each special character
    let slash_mask = _mm256_cmpeq_epi8(data, slash);
    let quote_mask = _mm256_cmpeq_epi8(data, quote);
    let tab_mask = _mm256_cmpeq_epi8(data, tab);
    let newline_mask = _mm256_cmpeq_epi8(data, newline);
    let carriage_mask = _mm256_cmpeq_epi8(data, carriage);
    let backspace_mask = _mm256_cmpeq_epi8(data, backspace);
    let formfeed_mask = _mm256_cmpeq_epi8(data, formfeed);

    // Check for control characters (< 0x20)
    // Note: AVX2 doesn't have unsigned comparison, so we use signed comparison
    // This works because ASCII control characters are all < 0x20 (positive signed values)
    let ctrl_mask = _mm256_cmpgt_epi8(ctrl_upper_bound, data);

    // Combine all masks
    let combined = _mm256_or_si256(
        _mm256_or_si256(
            _mm256_or_si256(slash_mask, quote_mask),
            _mm256_or_si256(tab_mask, newline_mask),
        ),
        _mm256_or_si256(
            _mm256_or_si256(carriage_mask, backspace_mask),
            _mm256_or_si256(formfeed_mask, ctrl_mask),
        ),
    );

    combined
}

#[inline(always)]
unsafe fn handle_block(src: &[u8], mask: &[u8; 32], dst: &mut Vec<u8>) {
    for (j, &m) in mask.iter().enumerate() {
        let c = src[j];
        if m == 0 {
            dst.push(c);
        } else {
            let escape_byte = ESCAPE[c as usize];
            if escape_byte != 0 {
                // Handle the escape
                dst.push(b'\\');
                if escape_byte == UU {
                    // Unicode escape for control characters
                    dst.extend_from_slice(b"u00");
                    let hex_digits = &HEX_BYTES[c as usize];
                    dst.push(hex_digits.0);
                    dst.push(hex_digits.1);
                } else {
                    // Simple escape
                    dst.push(escape_byte);
                }
            } else if c == b'\\' {
                // Backslash needs escaping
                dst.extend_from_slice(b"\\\\");
            } else {
                // Should not happen if mask is correct
                dst.push(c);
            }
        }
    }
}