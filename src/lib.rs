#![cfg_attr(feature = "nightly", feature(test))]

#[cfg(target_arch = "x86_64")]
pub use x86::encode_str;

#[cfg(target_arch = "x86_64")]
mod x86;

const BB: u8 = b'b'; // \x08
const TT: u8 = b't'; // \x09
const NN: u8 = b'n'; // \x0A
const FF: u8 = b'f'; // \x0C
const RR: u8 = b'r'; // \x0D
pub(crate) const QU: u8 = b'"'; // \x22
pub(crate) const BS: u8 = b'\\'; // \x5C
pub(crate) const UU: u8 = b'u'; // \x00...\x1F except the ones above
const __: u8 = 0;

// Lookup table of escape sequences. A value of b'x' at index i means that byte
// i is escaped as "\x" in JSON. A value of 0 means that byte i is not escaped.
pub(crate) const ESCAPE: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    UU, UU, UU, UU, UU, UU, UU, UU, BB, TT, NN, UU, FF, RR, UU, UU, // 0
    UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, // 1
    __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
    __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
];

// Precomputed hex byte pairs for faster control character escaping
pub(crate) const HEX_BYTES: [(u8, u8); 256] = {
    let mut bytes = [(0u8, 0u8); 256];
    let mut i = 0;
    while i < 256 {
        let high = (i >> 4) as u8;
        let low = (i & 0xF) as u8;
        bytes[i] = (
            if high < 10 {
                b'0' + high
            } else {
                b'a' + high - 10
            },
            if low < 10 {
                b'0' + low
            } else {
                b'a' + low - 10
            },
        );
        i += 1;
    }
    bytes
};

#[macro_export]
// We only use our own error type; no need for From conversions provided by the
// standard library's try! macro. This reduces lines of LLVM IR by 4%.
macro_rules! tri {
    ($e:expr $(,)?) => {
        match $e {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => return core::result::Result::Err(err),
        }
    };
}

#[inline]
pub fn encode_str_fallback<S: AsRef<str>>(input: S) -> String {
    let s = input.as_ref();
    let bytes = s.as_bytes();

    // Estimate capacity - most strings don't need much escaping
    // Add some padding for potential escapes
    let estimated_capacity = bytes.len() + bytes.len() / 2 + 2;
    let mut result = Vec::with_capacity(estimated_capacity);

    result.push(b'"');

    let mut start = 0;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Use lookup table to check if escaping is needed
        let escape_byte = ESCAPE[b as usize];

        if escape_byte == 0 {
            // No escape needed, continue scanning
            i += 1;
            continue;
        }

        // Copy any unescaped bytes before this position
        if start < i {
            result.extend_from_slice(&bytes[start..i]);
        }

        // Handle the escape
        result.push(b'\\');
        if escape_byte == UU {
            // Unicode escape for control characters
            result.extend_from_slice(b"u00");
            let hex_digits = &HEX_BYTES[b as usize];
            result.push(hex_digits.0);
            result.push(hex_digits.1);
        } else {
            // Simple escape
            result.push(escape_byte);
        }

        i += 1;
        start = i;
    }

    // Copy any remaining unescaped bytes
    if start < bytes.len() {
        result.extend_from_slice(&bytes[start..]);
    }

    result.push(b'"');

    // SAFETY: We only pushed valid UTF-8 bytes (original string bytes and ASCII escape sequences)
    unsafe { String::from_utf8_unchecked(result) }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    encode_str_fallback(input)
}

#[test]
fn test_escape_ascii_json_string() {
    let fixture = r#"abcdefghijklmnopqrstuvwxyz .*? hello world escape json string"#;
    assert_eq!(encode_str(fixture), serde_json::to_string(fixture).unwrap());
}

#[test]
fn test_escape_json_string() {
    let mut fixture = String::new();
    for i in 0u8..=0x1F {
        fixture.push(i as char);
    }
    fixture.push('\t');
    fixture.push('\x08');
    fixture.push('\x09');
    fixture.push('\x0A');
    fixture.push('\x0C');
    fixture.push('\x0D');
    fixture.push('\x22');
    fixture.push('\x5C');
    fixture.push_str("normal string");
    fixture.push('😊');
    fixture.push_str("中文 English 🚀 \n❓ 𝄞");
    encode_str(fixture.as_str());
    assert_eq!(
        encode_str(fixture.as_str()),
        serde_json::to_string(fixture.as_str()).unwrap(),
        "fixture: {:?}",
        fixture
    );
}
