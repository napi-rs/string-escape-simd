//! Optimized SIMD routines for escaping JSON strings.
//!
//! ## <div class="warning">Important</div>
//!
//! On aarch64 NEON hosts the available register width is **128** bits, which is narrower than the lookup table this implementation prefers. As a result the SIMD path may not outperform the generic fallback, which is reflected in the benchmark numbers below.
//!
//! On some modern macOS devices with larger register numbers, the SIMD path may outperform the generic fallback, see the [M3 max benchmark](#apple-m3-max) below.
//!
//! ### Note
//!
//! The `force_aarch64_neon` feature flag can be used to force use of the neon implementation on aarch64. This is useful for the benchmark.
//!
//! ## Benchmarks
//!
//! Numbers below come from `cargo bench` runs on GitHub Actions hardware. Criterion reports are summarized to make it easier to spot relative performance. "vs fastest" shows how much slower each implementation is compared to the fastest entry in the table (1.00× means fastest).
//!
//! ### GitHub Actions x86_64 (`ubuntu-latest`)
//!
//! `AVX2` enabled.
//!
//! **RxJS payload (~10k iterations)**
//!
//! | Implementation        | Median time   | vs fastest |
//! | --------------------- | ------------- | ---------- |
//! | **`escape simd`**     | **345.06 µs** | **1.00×**  |
//! | `escape v_jsonescape` | 576.25 µs     | 1.67×      |
//! | `escape generic`      | 657.94 µs     | 1.91×      |
//! | `serde_json`          | 766.72 µs     | 2.22×      |
//! | `json-escape`         | 782.65 µs     | 2.27×      |
//!
//! **Fixtures payload (~300 iterations)**
//!
//! | Implementation        | Median time  | vs fastest |
//! | --------------------- | ------------ | ---------- |
//! | **`escape simd`**     | **12.84 ms** | **1.00×**  |
//! | `escape v_jsonescape` | 19.66 ms     | 1.53×      |
//! | `escape generic`      | 22.53 ms     | 1.75×      |
//! | `serde_json`          | 24.65 ms     | 1.92×      |
//! | `json-escape`         | 26.64 ms     | 2.07×      |
//!
//! ### GitHub Actions aarch64 (`ubuntu-24.04-arm`)
//!
//! Neon enabled.
//!
//! **RxJS payload (~10k iterations)**
//!
//! | Implementation        | Median time   | vs fastest |
//! | --------------------- | ------------- | ---------- |
//! | **`escape generic`**  | **546.89 µs** | **1.00×**  |
//! | `escape simd`         | 589.29 µs     | 1.08×      |
//! | `serde_json`          | 612.33 µs     | 1.12×      |
//! | `json-escape`         | 624.66 µs     | 1.14×      |
//! | `escape v_jsonescape` | 789.14 µs     | 1.44×      |
//!
//! **Fixtures payload (~300 iterations)**
//!
//! | Implementation        | Median time  | vs fastest |
//! | --------------------- | ------------ | ---------- |
//! | **`escape generic`**  | **17.81 ms** | **1.00×**  |
//! | `serde_json`          | 19.77 ms     | 1.11×      |
//! | `json-escape`         | 20.84 ms     | 1.17×      |
//! | `escape simd`         | 21.04 ms     | 1.18×      |
//! | `escape v_jsonescape` | 25.57 ms     | 1.44×      |
//!
//! ### GitHub Actions macOS (`macos-latest`)
//!
//! Apple M1 chip
//!
//! **RxJS payload (~10k iterations)**
//!
//! | Implementation        | Median time   | vs fastest |
//! | --------------------- | ------------- | ---------- |
//! | **`escape generic`**  | **759.07 µs** | **1.00×**  |
//! | `escape simd`         | 764.98 µs     | 1.01×      |
//! | `serde_json`          | 793.91 µs     | 1.05×      |
//! | `json-escape`         | 868.21 µs     | 1.14×      |
//! | `escape v_jsonescape` | 926.00 µs     | 1.22×      |
//!
//! **Fixtures payload (~300 iterations)**
//!
//! | Implementation        | Median time  | vs fastest |
//! | --------------------- | ------------ | ---------- |
//! | **`serde_json`**      | **26.41 ms** | **1.00×**  |
//! | `escape generic`      | 26.43 ms     | 1.00×      |
//! | `escape simd`         | 26.42 ms     | 1.00×      |
//! | `json-escape`         | 28.94 ms     | 1.10×      |
//! | `escape v_jsonescape` | 29.22 ms     | 1.11×      |
//!
//! ### Apple M3 Max
//!
//! **RxJS payload (~10k iterations)**
//!
//! | Implementation        | Median time   | vs fastest |
//! | --------------------- | ------------- | ---------- |
//! | **`escape simd`**     | **307.20 µs** | **1.00×**  |
//! | `escape generic`      | 490.00 µs     | 1.60×      |
//! | `serde_json`          | 570.35 µs     | 1.86×      |
//! | `escape v_jsonescape` | 599.72 µs     | 1.95×      |
//! | `json-escape`         | 644.73 µs     | 2.10×      |
//!
//! **Fixtures payload (~300 iterations)**
//!
//! | Implementation        | Median time  | vs fastest |
//! | --------------------- | ------------ | ---------- |
//! | **`escape generic`**  | **17.89 ms** | **1.00×**  |
//! | **`escape simd`**     | **17.92 ms** | **1.00×**  |
//! | `serde_json`          | 19.78 ms     | 1.11×      |
//! | `escape v_jsonescape` | 21.09 ms     | 1.18×      |
//! | `json-escape`         | 22.43 ms     | 1.25×      |

#[cfg(target_arch = "aarch64")]
mod aarch64;
mod generic;
#[cfg(target_arch = "x86_64")]
mod x86;

pub use generic::escape_generic;

/// Main entry point for JSON string escaping with SIMD acceleration
/// If the platform is supported, the SIMD path will be used. Otherwise, the generic fallback will be used.
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
        #[cfg(feature = "force_aarch64_neon")]
        {
            return aarch64::escape_neon(input);
        }
        #[cfg(not(feature = "force_aarch64_neon"))]
        {
            // on Apple M2 and later, the `bf16` feature is available
            // it means they have more registers and can significantly benefit from the SIMD path
            // TODO: add support for sve2 chips with wider registers
            // github actions ubuntu-24.04-arm runner has 128 bits sve2 registers, it's not enough for the SIMD path
            if cfg!(target_os = "macos") && std::arch::is_aarch64_feature_detected!("bf16") {
                return aarch64::escape_neon(input);
            } else {
                return escape_generic(input);
            }
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
