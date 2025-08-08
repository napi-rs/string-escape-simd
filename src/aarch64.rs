/*!
 * High-performance JSON string escaping using V8-style SIMD optimizations for aarch64.
 * 
 * Core V8 insight: Optimize for the common case where most data needs NO escaping.
 * Use SIMD for fast detection, bulk copy for clean chunks, scalar fallback for dirty chunks.
 */

use std::arch::aarch64::{
    vceqq_u8, vdupq_n_u8, vld1q_u8_x4, vmaxvq_u8, vorrq_u8, vcltq_u8,
};

use crate::encode_str_inner;

/// Process 64 bytes per check - optimal for cache and SIMD  
const CHUNK: usize = 64;

/// Ultra-fast SIMD check: does this 64-byte chunk need ANY escaping?
/// Returns true if completely clean (bulk copy safe)
#[inline(always)]
unsafe fn chunk_is_clean(ptr: *const u8) -> bool {
    let quad = vld1q_u8_x4(ptr);
    
    // Check for escape characters in all four 16-byte vectors
    // Characters needing escape: < 0x20, == 0x22 ("), == 0x5C (\)
    let needs_escape_0 = vorrq_u8(
        vcltq_u8(quad.0, vdupq_n_u8(0x20)),
        vorrq_u8(vceqq_u8(quad.0, vdupq_n_u8(0x22)), vceqq_u8(quad.0, vdupq_n_u8(0x5C)))
    );
    let needs_escape_1 = vorrq_u8(
        vcltq_u8(quad.1, vdupq_n_u8(0x20)),
        vorrq_u8(vceqq_u8(quad.1, vdupq_n_u8(0x22)), vceqq_u8(quad.1, vdupq_n_u8(0x5C)))
    );
    let needs_escape_2 = vorrq_u8(
        vcltq_u8(quad.2, vdupq_n_u8(0x20)),
        vorrq_u8(vceqq_u8(quad.2, vdupq_n_u8(0x22)), vceqq_u8(quad.2, vdupq_n_u8(0x5C)))
    );
    let needs_escape_3 = vorrq_u8(
        vcltq_u8(quad.3, vdupq_n_u8(0x20)),
        vorrq_u8(vceqq_u8(quad.3, vdupq_n_u8(0x22)), vceqq_u8(quad.3, vdupq_n_u8(0x5C)))
    );
    
    // Combine all masks and check if ANY byte needs escaping
    let all_masks = vorrq_u8(
        vorrq_u8(needs_escape_0, needs_escape_1),
        vorrq_u8(needs_escape_2, needs_escape_3)
    );
    
    // Return true if NO bytes need escaping (chunk is clean)
    vmaxvq_u8(all_masks) == 0
}

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let bytes = s.as_bytes();
    let n = bytes.len();
    
    // Simple capacity estimation
    let mut out = Vec::with_capacity(n + n / 16 + 2);
    out.push(b'"');

    // V8-style optimization: Focus on the fast path for clean data
    unsafe {
        let mut i = 0;
        let mut clean_start = 0;
        
        // Process in 64-byte chunks optimized for clean data
        while i + CHUNK <= n {
            let ptr = bytes.as_ptr().add(i);
            
            if chunk_is_clean(ptr) {
                // Clean chunk - continue scanning
                i += CHUNK;
            } else {
                // Found dirty chunk - flush any accumulated clean data first
                if clean_start < i {
                    out.extend_from_slice(&bytes[clean_start..i]);
                }
                
                // Process this single dirty chunk with proven scalar code
                encode_str_inner(&bytes[i..i + CHUNK], &mut out);
                i += CHUNK;
                clean_start = i;
            }
        }
        
        // Flush any remaining clean data
        if clean_start < i {
            out.extend_from_slice(&bytes[clean_start..i]);
        }
        
        // Handle remaining bytes (less than CHUNK)
        if i < n {
            encode_str_inner(&bytes[i..], &mut out);
        }
    }
    
    out.push(b'"');
    // SAFETY: we only emit valid UTF-8
    unsafe { String::from_utf8_unchecked(out) }
}
