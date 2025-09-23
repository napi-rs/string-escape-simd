use json_escape_simd::{encode_str_fallback, escape};

fn main() {
    let fixture = include_str!("../cal.com.tsx");
    let encoded = escape(fixture);
    let encoded_fallback = encode_str_fallback(fixture);
    assert_eq!(encoded, encoded_fallback);
}
