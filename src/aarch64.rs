use std::arch::aarch64::{
    uint8x16_t, uint8x16x4_t, vld1q_u8, vld1q_u8_x4, vmaxvq_u8, vpmaxq_u8, vqtbl1q_u8, vqtbl4q_u8,
};
use std::sync::OnceLock;

use crate::{encode_str_inner, ESCAPE};

static ESCAPE_TABLE_LOW: OnceLock<uint8x16x4_t> = OnceLock::new();
static ESCAPE_TABLE_HIGH: OnceLock<uint8x16_t> = OnceLock::new();
const CHUNK_SIZE: usize = 16;

pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    let input_str = input.as_ref();
    let mut output = Vec::with_capacity(input_str.len() + 2);
    let bytes = input_str.as_bytes();
    let writer = &mut output;
    writer.push(b'"');
    unsafe {
        let mut start = 0;
        while start + CHUNK_SIZE < bytes.len() {
            let next_chunk = start + CHUNK_SIZE;
            let table_low = ESCAPE_TABLE_LOW.get_or_init(|| vld1q_u8_x4(ESCAPE[0..64].as_ptr()));
            let table_high = ESCAPE_TABLE_HIGH.get_or_init(|| vld1q_u8(ESCAPE[64..80].as_ptr()));
            let chunk = vld1q_u8(bytes[start..next_chunk].as_ptr());
            let low_mask = vqtbl4q_u8(*table_low, chunk);
            let high_mask = vqtbl1q_u8(*table_high, chunk);
            if vmaxvq_u8(vpmaxq_u8(low_mask, high_mask)) == 0 {
                writer.extend_from_slice(&bytes[start..next_chunk]);
                start = next_chunk;
                continue;
            }
            encode_str_inner(&bytes[start..next_chunk], writer);
            start = next_chunk;
        }

        if start < bytes.len() {
            encode_str_inner(&bytes[start..], writer);
        }
    }
    writer.push(b'"');
    // Safety: the bytes are valid UTF-8
    unsafe { String::from_utf8_unchecked(output) }
}
