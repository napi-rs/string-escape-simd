use string_escape_simd::{encode_str, encode_str_fallback};

fn main() {
    let fixture = include_str!("../cal.com.tsx");
    let encoded = encode_str(fixture);
    let encoded_fallback = encode_str_fallback(fixture);
    assert_eq!(encoded, encoded_fallback);
}
