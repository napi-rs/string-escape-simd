use json_escape_simd::{escape, escape_generic};

fn main() {
    let fixture = include_str!("../cal.com.tsx");
    let encoded = escape(fixture);
    let encoded_fallback = escape_generic(fixture);
    assert_eq!(encoded, encoded_fallback);
}
