#![cfg_attr(feature = "nightly", feature(test))]

#[cfg(target_arch = "aarch64")]
pub use aarch64::encode_str;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64::encode_str;

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

pub(crate) const REVERSE_SOLIDUS: &[u8; 2] = b"\\\\";

/// Represents a character escape code in a type-safe manner.
#[repr(u8)]
pub enum CharEscape {
    /// An escaped quote `"`
    Quote,
    /// An escaped reverse solidus `\`
    ReverseSolidus,
    /// An escaped solidus `/`
    Solidus,
    /// An escaped backspace character (usually escaped as `\b`)
    Backspace,
    /// An escaped form feed character (usually escaped as `\f`)
    FormFeed,
    /// An escaped line feed character (usually escaped as `\n`)
    LineFeed,
    /// An escaped carriage return character (usually escaped as `\r`)
    CarriageReturn,
    /// An escaped tab character (usually escaped as `\t`)
    Tab,
    /// An escaped ASCII plane control character (usually escaped as
    /// `\u00XX` where `XX` are two hex characters)
    AsciiControl(u8),
}

impl CharEscape {
    #[inline]
    fn from_escape_table(escape: u8, byte: u8) -> CharEscape {
        match escape {
            self::BB => CharEscape::Backspace,
            self::TT => CharEscape::Tab,
            self::NN => CharEscape::LineFeed,
            self::FF => CharEscape::FormFeed,
            self::RR => CharEscape::CarriageReturn,
            self::QU => CharEscape::Quote,
            self::BS => CharEscape::ReverseSolidus,
            self::UU => CharEscape::AsciiControl(byte),
            _ => unreachable!("Invalid escape code: {}", escape),
        }
    }
}

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

#[cfg_attr(target_arch = "aarch64", allow(unused))]
#[inline]
pub fn encode_str_fallback<S: AsRef<str>>(input: S) -> String {
    let mut output = String::with_capacity(input.as_ref().len() + 2);
    let writer = unsafe { output.as_mut_vec() };
    writer.push(b'"');
    encode_str_inner(input.as_ref().as_bytes(), writer);
    writer.push(b'"');
    output
}

#[cfg(all(not(target_arch = "aarch64"), not(target_arch = "x86_64")))]
pub fn encode_str<S: AsRef<str>>(input: S) -> String {
    encode_str_fallback(input)
}

#[inline]
pub(crate) fn encode_str_inner(bytes: &[u8], writer: &mut Vec<u8>) {
    let mut start = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        let escape = ESCAPE[byte as usize];
        if escape == 0 {
            continue;
        }

        if start < i {
            writer.extend_from_slice(&bytes[start..i]);
        }

        let char_escape = CharEscape::from_escape_table(escape, byte);
        write_char_escape(writer, char_escape);

        start = i + 1;
    }

    if start == bytes.len() {
        return;
    }
    writer.extend_from_slice(&bytes[start..]);
}

/// Writes a character escape code to the specified writer.
#[inline]
fn write_char_escape(writer: &mut Vec<u8>, char_escape: CharEscape) {
    use self::CharEscape::*;

    let s = match char_escape {
        Quote => b"\\\"",
        ReverseSolidus => REVERSE_SOLIDUS,
        Solidus => b"\\/",
        Backspace => b"\\b",
        FormFeed => b"\\f",
        LineFeed => b"\\n",
        CarriageReturn => b"\\r",
        Tab => b"\\t",
        AsciiControl(byte) => {
            static HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";
            let bytes = &[
                b'\\',
                b'u',
                b'0',
                b'0',
                HEX_DIGITS[(byte >> 4) as usize],
                HEX_DIGITS[(byte & 0xF) as usize],
            ];
            return writer.extend_from_slice(bytes);
        }
    };

    writer.extend_from_slice(s)
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


#[test]
fn test() {
    let x = ESCAPE[b'\\' as usize];
    println!("{x}")
}