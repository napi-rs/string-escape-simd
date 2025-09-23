#[cfg(target_arch = "x86_64")]
mod x86;

#[cfg(all(target_arch = "aarch64", not(feature = "force_aarch64_generic")))]
mod aarch64;

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
pub fn escape_generic<S: AsRef<str>>(input: S) -> String {
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

/// Main entry point for JSON string escaping with SIMD acceleration
pub fn escape<S: AsRef<str>>(input: S) -> String {
    #[cfg(target_arch = "x86_64")]
    {
        // Runtime CPU feature detection for x86_64
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            unsafe { return x86::escape_avx512(input) }
        } else if is_x86_feature_detected!("avx2") {
            unsafe { return x86::escape_avx2(input) }
        } else if is_x86_feature_detected!("sse2") {
            unsafe { return x86::escape_sse2(input) }
        } else {
            return escape_generic(input);
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        #[cfg(feature = "force_aarch64_generic")]
        {
            return escape_generic(input);
        }
        #[cfg(not(feature = "force_aarch64_generic"))]
        {
            return aarch64::escape_neon(input);
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    escape_generic(input)
}

#[test]
fn test_escape_ascii_json_string() {
    let fixture = r#"abcdefghijklmnopqrstuvwxyz .*? hello world escape json string"#;
    assert_eq!(escape(fixture), serde_json::to_string(fixture).unwrap());
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
    escape(fixture.as_str());
    assert_eq!(
        escape(fixture.as_str()),
        serde_json::to_string(fixture.as_str()).unwrap(),
        "fixture: {:?}",
        fixture
    );
}

// Test cases for various string sizes to cover different SIMD paths

#[test]
fn test_empty_string() {
    assert_eq!(escape(""), r#""""#);
}

#[test]
fn test_very_small_strings() {
    // Less than 16 bytes (SSE register size)
    assert_eq!(escape("a"), r#""a""#);
    assert_eq!(escape("ab"), r#""ab""#);
    assert_eq!(escape("hello"), r#""hello""#);
    assert_eq!(escape("hello\n"), r#""hello\n""#);
    assert_eq!(escape("\""), r#""\"""#);
    assert_eq!(escape("\\"), r#""\\""#);
    assert_eq!(escape("\t"), r#""\t""#);
    assert_eq!(escape("\r\n"), r#""\r\n""#);
}

#[test]
fn test_small_strings_16_bytes() {
    // Exactly 16 bytes - SSE register boundary
    let s16 = "0123456789abcdef";
    assert_eq!(s16.len(), 16);
    assert_eq!(escape(s16), serde_json::to_string(s16).unwrap());

    // 16 bytes with escapes
    let s16_esc = "01234567\t9abcde";
    assert_eq!(s16_esc.len(), 15); // \t is 1 byte
    assert_eq!(escape(s16_esc), serde_json::to_string(s16_esc).unwrap());
}

#[test]
fn test_medium_strings_32_bytes() {
    // Exactly 32 bytes - AVX2 register boundary
    let s32 = "0123456789abcdef0123456789abcdef";
    assert_eq!(s32.len(), 32);
    assert_eq!(escape(s32), serde_json::to_string(s32).unwrap());

    // 32 bytes with escapes at different positions
    let s32_esc = "0123456789abcde\"0123456789abcde";
    assert_eq!(escape(s32_esc), serde_json::to_string(s32_esc).unwrap());
}

#[test]
fn test_large_strings_128_bytes() {
    // Exactly 128 bytes - main loop size
    let s128 = "0123456789abcdef".repeat(8);
    assert_eq!(s128.len(), 128);
    assert_eq!(escape(&s128), serde_json::to_string(&s128).unwrap());

    // 128 bytes with escapes spread throughout
    let mut s128_esc = String::new();
    for i in 0..8 {
        if i % 2 == 0 {
            s128_esc.push_str("0123456789abcd\n");
        } else {
            s128_esc.push_str("0123456789abcd\"");
        }
    }
    assert_eq!(escape(&s128_esc), serde_json::to_string(&s128_esc).unwrap());
}

#[test]
fn test_unaligned_data() {
    // Test strings that start at various alignments
    for offset in 0..32 {
        let padding = " ".repeat(offset);
        let test_str = format!("{}{}", padding, "test\nstring\"with\\escapes");
        let result = escape(&test_str[offset..]);
        let expected = serde_json::to_string(&test_str[offset..]).unwrap();
        assert_eq!(result, expected, "Failed at offset {}", offset);
    }
}

#[test]
fn test_sparse_escapes() {
    // Large string with escapes only at the beginning and end
    let mut s = String::new();
    s.push('"');
    s.push_str(&"a".repeat(500));
    s.push('\\');
    assert_eq!(escape(&s), serde_json::to_string(&s).unwrap());
}

#[test]
fn test_dense_escapes() {
    // String with many escapes
    let s = "\"\\\"\\\"\\\"\\".repeat(50);
    assert_eq!(escape(&s), serde_json::to_string(&s).unwrap());

    // All control characters
    let mut ctrl = String::new();
    for _ in 0..10 {
        for i in 0u8..32 {
            ctrl.push(i as char);
        }
    }
    assert_eq!(escape(&ctrl), serde_json::to_string(&ctrl).unwrap());
}

#[test]
fn test_boundary_conditions() {
    // Test around 256 byte boundary (common cache line multiple)
    for size in 250..260 {
        let s = "a".repeat(size);
        assert_eq!(escape(&s), serde_json::to_string(&s).unwrap());

        // With escape at the end
        let mut s_esc = "a".repeat(size - 1);
        s_esc.push('"');
        assert_eq!(escape(&s_esc), serde_json::to_string(&s_esc).unwrap());
    }
}

#[test]
fn test_all_escape_types() {
    // Test each escape type individually
    assert_eq!(escape("\x00"), r#""\u0000""#);
    assert_eq!(escape("\x08"), r#""\b""#);
    assert_eq!(escape("\x09"), r#""\t""#);
    assert_eq!(escape("\x0A"), r#""\n""#);
    assert_eq!(escape("\x0C"), r#""\f""#);
    assert_eq!(escape("\x0D"), r#""\r""#);
    assert_eq!(escape("\x1F"), r#""\u001f""#);
    assert_eq!(escape("\""), r#""\"""#);
    assert_eq!(escape("\\"), r#""\\""#);

    // Test all control characters
    for i in 0u8..32 {
        let s = String::from_utf8(vec![i]).unwrap();
        let result = escape(&s);
        let expected = serde_json::to_string(&s).unwrap();
        assert_eq!(result, expected, "Failed for byte 0x{:02x}", i);
    }
}

#[test]
fn test_mixed_content() {
    // Mix of ASCII, escapes, and multi-byte UTF-8
    let mixed = r#"Hello "World"!
    Tab:	Here
    Emoji: 😀 Chinese: 中文
    Math: ∑∫∂ Music: 𝄞
    Escape: \" \\ \n \r \t"#;
    assert_eq!(escape(mixed), serde_json::to_string(mixed).unwrap());
}

#[test]
fn test_repeated_patterns() {
    // Patterns that might benefit from or confuse SIMD operations
    let pattern1 = "abcd".repeat(100);
    assert_eq!(escape(&pattern1), serde_json::to_string(&pattern1).unwrap());

    let pattern2 = "a\"b\"".repeat(100);
    assert_eq!(escape(&pattern2), serde_json::to_string(&pattern2).unwrap());

    let pattern3 = "\t\n".repeat(100);
    assert_eq!(escape(&pattern3), serde_json::to_string(&pattern3).unwrap());
}

#[test]
fn test_rxjs() {
    let dir = glob::glob("node_modules/rxjs/src/**/*.ts").unwrap();
    let mut sources = Vec::new();
    for entry in dir {
        sources.push(std::fs::read_to_string(entry.unwrap()).unwrap());
    }
    assert!(!sources.is_empty());
    for source in sources {
        assert_eq!(escape(&source), serde_json::to_string(&source).unwrap());
    }
}

#[test]
fn test_sources() {
    let ts_paths = glob::glob("fixtures/**/*.ts").unwrap();
    let tsx_paths = glob::glob("fixtures/**/*.tsx").unwrap();
    let js_paths = glob::glob("fixtures/**/*.js").unwrap();
    let mjs_paths = glob::glob("fixtures/**/*.mjs").unwrap();
    let cjs_paths = glob::glob("fixtures/**/*.cjs").unwrap();
    let mut sources = Vec::new();
    for entry in ts_paths
        .chain(tsx_paths)
        .chain(js_paths)
        .chain(mjs_paths)
        .chain(cjs_paths)
    {
        let p = entry.unwrap();
        if std::fs::metadata(&p).unwrap().is_file() {
            sources.push(std::fs::read_to_string(&p).unwrap());
        }
    }
    assert!(!sources.is_empty());
    for source in sources {
        assert_eq!(escape(&source), serde_json::to_string(&source).unwrap());
    }
}
