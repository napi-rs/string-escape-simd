use string_escape_simd::encode_str;

fn main() {
    let fixture = include_str!("../cal.com.tsx");
    encode_str(fixture);
}
