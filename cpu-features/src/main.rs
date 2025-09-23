#![cfg_attr(
    all(target_arch = "aarch64", feature = "nightly"),
    feature(stdarch_aarch64_feature_detection)
)]
#![cfg_attr(
    all(target_arch = "x86_64", feature = "nightly"),
    feature(xop_target_feature)
)]
#![cfg_attr(
    all(target_arch = "x86_64", feature = "nightly"),
    feature(movrs_target_feature)
)]
#![cfg_attr(
    all(target_arch = "x86_64", feature = "nightly"),
    feature(x86_amx_intrinsics)
)]

use std::fmt::{self, Write as _};

#[cfg(target_arch = "aarch64")]
use std::arch::asm;

#[derive(Clone, Copy)]
enum FeatureStatus {
    Available(bool),
    Unsupported(&'static str),
}

#[cfg(target_arch = "x86_64")]
fn main() {
    const FEATURE_ORDER: &[&str] = &[
        "aes",
        "pclmulqdq",
        "rdrand",
        "rdseed",
        "tsc",
        "mmx",
        "sse",
        "sse2",
        "sse3",
        "ssse3",
        "sse4.1",
        "sse4.2",
        "sse4a",
        "sha",
        "avx",
        "avx2",
        "sha512",
        "sm3",
        "sm4",
        "avx512f",
        "avx512cd",
        "avx512er",
        "avx512pf",
        "avx512bw",
        "avx512dq",
        "avx512vl",
        "avx512ifma",
        "avx512vbmi",
        "avx512vpopcntdq",
        "avx512vbmi2",
        "gfni",
        "vaes",
        "vpclmulqdq",
        "avx512vnni",
        "avx512bitalg",
        "avx512bf16",
        "avx512vp2intersect",
        "avx512fp16",
        "avxvnni",
        "avxifma",
        "avxneconvert",
        "avxvnniint8",
        "avxvnniint16",
        "amx-tile",
        "amx-int8",
        "amx-bf16",
        "amx-fp16",
        "amx-complex",
        "amx-avx512",
        "amx-fp8",
        "amx-movrs",
        "amx-tf32",
        "amx-transpose",
        "f16c",
        "fma",
        "bmi1",
        "bmi2",
        "abm",
        "lzcnt",
        "tbm",
        "popcnt",
        "fxsr",
        "xsave",
        "xsaveopt",
        "xsaves",
        "xsavec",
        "cmpxchg16b",
        "kl",
        "widekl",
        "adx",
        "rtm",
        "movbe",
        "ermsb",
        "movrs",
        "xop",
    ];

    let nightly = cfg!(feature = "nightly");
    let features: Vec<(&str, FeatureStatus)> = FEATURE_ORDER
        .iter()
        .map(|&name| (name, detect_x86_feature(name, nightly)))
        .collect();

    let max_width_bits = if std::arch::is_x86_feature_detected!("avx512f") {
        "512 bits".to_string()
    } else if std::arch::is_x86_feature_detected!("avx") {
        "256 bits".to_string()
    } else if std::arch::is_x86_feature_detected!("sse") {
        "128 bits".to_string()
    } else {
        "64 bits".to_string()
    };

    print_report("x86_64", &max_width_bits, &features);
}

#[cfg(target_arch = "aarch64")]
fn main() {
    const FEATURE_ORDER: &[&str] = &[
        "aes",
        "asimd",
        "neon",
        "bf16",
        "bti",
        "crc",
        "cssc",
        "dit",
        "dotprod",
        "dpb",
        "dpb2",
        "ecv",
        "f32mm",
        "f64mm",
        "faminmax",
        "fcma",
        "fhm",
        "flagm",
        "flagm2",
        "fp",
        "fp16",
        "fp8",
        "fp8dot2",
        "fp8dot4",
        "fp8fma",
        "fpmr",
        "frintts",
        "hbc",
        "i8mm",
        "jsconv",
        "lse",
        "lse128",
        "lse2",
        "lut",
        "mops",
        "mte",
        "paca",
        "pacg",
        "pauth-lr",
        "pmull",
        "rand",
        "rcpc",
        "rcpc2",
        "rcpc3",
        "rdm",
        "sb",
        "sha2",
        "sha3",
        "sm4",
        "sme",
        "sme-b16b16",
        "sme-f16f16",
        "sme-f64f64",
        "sme-f8f16",
        "sme-f8f32",
        "sme-fa64",
        "sme-i16i64",
        "sme-lutv2",
        "sme2",
        "sme2p1",
        "ssbs",
        "ssve-fp8dot2",
        "ssve-fp8dot4",
        "ssve-fp8fma",
        "sve",
        "sve-b16b16",
        "sve2",
        "sve2-aes",
        "sve2-bitperm",
        "sve2-sha3",
        "sve2-sm4",
        "sve2p1",
        "tme",
        "wfxt",
    ];

    let nightly = cfg!(feature = "nightly");
    let features: Vec<(&str, FeatureStatus)> = FEATURE_ORDER
        .iter()
        .map(|&name| (name, detect_aarch64_feature(name, nightly)))
        .collect();

    let simd_width = if std::arch::is_aarch64_feature_detected!("sve") {
        query_sve_vector_length_bits()
            .map(|bits| format!("{} bits (via CNTB)", bits))
            .unwrap_or_else(|| "variable (SVE runtime length)".to_string())
    } else if std::arch::is_aarch64_feature_detected!("neon")
        || std::arch::is_aarch64_feature_detected!("asimd")
    {
        "128 bits".to_string()
    } else {
        "No SIMD detected".to_string()
    };

    print_report("aarch64", &simd_width, &features);
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn main() {
    eprintln!("cpu_features example is only implemented for x86_64 and aarch64 targets.");
}

fn print_report(arch: &str, width_bits: &str, features: &[(&str, FeatureStatus)]) {
    let mut buffer = String::new();
    writeln!(buffer, "Architecture: {}", arch).unwrap();
    writeln!(buffer, "Max SIMD register width: {}", width_bits).unwrap();
    write_features(&mut buffer, features).unwrap();
    print!("{}", buffer);
}

fn write_features(buffer: &mut String, features: &[(&str, FeatureStatus)]) -> fmt::Result {
    writeln!(buffer, "Detected SIMD features:")?;
    for (name, status) in features {
        match status {
            FeatureStatus::Available(true) => writeln!(buffer, "  - {:<16} yes", name)?,
            FeatureStatus::Available(false) => writeln!(buffer, "  - {:<16} no", name)?,
            FeatureStatus::Unsupported(reason) => {
                writeln!(buffer, "  - {:<16} n/a ({})", name, reason)?
            }
        }
    }
    Ok(())
}

#[cfg(target_arch = "x86_64")]
fn detect_x86_feature(name: &str, is_nightly: bool) -> FeatureStatus {
    use FeatureStatus::{Available, Unsupported};
    const NIGHTLY_MSG: &str = "requires nightly Rust (stdarch_x86_feature_detection)";

    if let Some(present) = detect_stable_x86_feature(name) {
        return Available(present);
    }

    if let Some(present) = detect_unstable_x86_feature(name, is_nightly) {
        return Available(present);
    }

    Unsupported(match name {
        // If we reach here, the feature is either unknown or needs nightly.
        _ if is_nightly => "unknown feature name",
        _ => NIGHTLY_MSG,
    })
}

#[cfg(target_arch = "x86_64")]
fn detect_stable_x86_feature(name: &str) -> Option<bool> {
    Some(match name {
        "aes" => std::arch::is_x86_feature_detected!("aes"),
        "pclmulqdq" => std::arch::is_x86_feature_detected!("pclmulqdq"),
        "rdrand" => std::arch::is_x86_feature_detected!("rdrand"),
        "rdseed" => std::arch::is_x86_feature_detected!("rdseed"),
        "tsc" => std::arch::is_x86_feature_detected!("tsc"),
        "mmx" => std::arch::is_x86_feature_detected!("mmx"),
        "sse" => std::arch::is_x86_feature_detected!("sse"),
        "sse2" => std::arch::is_x86_feature_detected!("sse2"),
        "sse3" => std::arch::is_x86_feature_detected!("sse3"),
        "ssse3" => std::arch::is_x86_feature_detected!("ssse3"),
        "sse4.1" => std::arch::is_x86_feature_detected!("sse4.1"),
        "sse4.2" => std::arch::is_x86_feature_detected!("sse4.2"),
        "sse4a" => std::arch::is_x86_feature_detected!("sse4a"),
        "sha" => std::arch::is_x86_feature_detected!("sha"),
        "avx" => std::arch::is_x86_feature_detected!("avx"),
        "avx2" => std::arch::is_x86_feature_detected!("avx2"),
        "sha512" => std::arch::is_x86_feature_detected!("sha512"),
        "sm3" => std::arch::is_x86_feature_detected!("sm3"),
        "sm4" => std::arch::is_x86_feature_detected!("sm4"),
        "avx512f" => std::arch::is_x86_feature_detected!("avx512f"),
        "avx512cd" => std::arch::is_x86_feature_detected!("avx512cd"),
        "avx512er" => std::arch::is_x86_feature_detected!("avx512er"),
        "avx512pf" => std::arch::is_x86_feature_detected!("avx512pf"),
        "avx512bw" => std::arch::is_x86_feature_detected!("avx512bw"),
        "avx512dq" => std::arch::is_x86_feature_detected!("avx512dq"),
        "avx512vl" => std::arch::is_x86_feature_detected!("avx512vl"),
        "avx512ifma" => std::arch::is_x86_feature_detected!("avx512ifma"),
        "avx512vbmi" => std::arch::is_x86_feature_detected!("avx512vbmi"),
        "avx512vpopcntdq" => std::arch::is_x86_feature_detected!("avx512vpopcntdq"),
        "avx512vbmi2" => std::arch::is_x86_feature_detected!("avx512vbmi2"),
        "gfni" => std::arch::is_x86_feature_detected!("gfni"),
        "vaes" => std::arch::is_x86_feature_detected!("vaes"),
        "vpclmulqdq" => std::arch::is_x86_feature_detected!("vpclmulqdq"),
        "avx512vnni" => std::arch::is_x86_feature_detected!("avx512vnni"),
        "avx512bitalg" => std::arch::is_x86_feature_detected!("avx512bitalg"),
        "avx512bf16" => std::arch::is_x86_feature_detected!("avx512bf16"),
        "avx512vp2intersect" => std::arch::is_x86_feature_detected!("avx512vp2intersect"),
        "avx512fp16" => std::arch::is_x86_feature_detected!("avx512fp16"),
        "avxvnni" => std::arch::is_x86_feature_detected!("avxvnni"),
        "avxifma" => std::arch::is_x86_feature_detected!("avxifma"),
        "avxneconvert" => std::arch::is_x86_feature_detected!("avxneconvert"),
        "avxvnniint8" => std::arch::is_x86_feature_detected!("avxvnniint8"),
        "avxvnniint16" => std::arch::is_x86_feature_detected!("avxvnniint16"),
        "f16c" => std::arch::is_x86_feature_detected!("f16c"),
        "fma" => std::arch::is_x86_feature_detected!("fma"),
        "bmi1" => std::arch::is_x86_feature_detected!("bmi1"),
        "bmi2" => std::arch::is_x86_feature_detected!("bmi2"),
        "abm" => std::arch::is_x86_feature_detected!("lzcnt"),
        "lzcnt" => std::arch::is_x86_feature_detected!("lzcnt"),
        "tbm" => std::arch::is_x86_feature_detected!("tbm"),
        "popcnt" => std::arch::is_x86_feature_detected!("popcnt"),
        "fxsr" => std::arch::is_x86_feature_detected!("fxsr"),
        "xsave" => std::arch::is_x86_feature_detected!("xsave"),
        "xsaveopt" => std::arch::is_x86_feature_detected!("xsaveopt"),
        "xsaves" => std::arch::is_x86_feature_detected!("xsaves"),
        "xsavec" => std::arch::is_x86_feature_detected!("xsavec"),
        "cmpxchg16b" => std::arch::is_x86_feature_detected!("cmpxchg16b"),
        "kl" => std::arch::is_x86_feature_detected!("kl"),
        "widekl" => std::arch::is_x86_feature_detected!("widekl"),
        "adx" => std::arch::is_x86_feature_detected!("adx"),
        "rtm" => std::arch::is_x86_feature_detected!("rtm"),
        "movbe" => std::arch::is_x86_feature_detected!("movbe"),
        "ermsb" => std::arch::is_x86_feature_detected!("ermsb"),
        _ => return None,
    })
}

#[cfg(target_arch = "x86_64")]
fn detect_unstable_x86_feature(name: &str, is_nightly: bool) -> Option<bool> {
    if !is_nightly {
        return None;
    }
    detect_unstable_x86_feature_impl(name)
}

#[cfg(all(target_arch = "x86_64", feature = "nightly"))]
fn detect_unstable_x86_feature_impl(name: &str) -> Option<bool> {
    Some(match name {
        "amx-tile" => std::arch::is_x86_feature_detected!("amx-tile"),
        "amx-int8" => std::arch::is_x86_feature_detected!("amx-int8"),
        "amx-bf16" => std::arch::is_x86_feature_detected!("amx-bf16"),
        "amx-fp16" => std::arch::is_x86_feature_detected!("amx-fp16"),
        "amx-complex" => std::arch::is_x86_feature_detected!("amx-complex"),
        "amx-avx512" => std::arch::is_x86_feature_detected!("amx-avx512"),
        "amx-fp8" => std::arch::is_x86_feature_detected!("amx-fp8"),
        "amx-movrs" => std::arch::is_x86_feature_detected!("amx-movrs"),
        "amx-tf32" => std::arch::is_x86_feature_detected!("amx-tf32"),
        "amx-transpose" => std::arch::is_x86_feature_detected!("amx-transpose"),
        "movrs" => std::arch::is_x86_feature_detected!("movrs"),
        "xop" => std::arch::is_x86_feature_detected!("xop"),
        _ => return None,
    })
}

#[cfg(all(target_arch = "x86_64", not(feature = "nightly")))]
fn detect_unstable_x86_feature_impl(_name: &str) -> Option<bool> {
    None
}

#[cfg(target_arch = "aarch64")]
fn detect_aarch64_feature(name: &str, is_nightly: bool) -> FeatureStatus {
    use FeatureStatus::{Available, Unsupported};
    const NIGHTLY_MSG: &str = "requires nightly Rust (stdarch_aarch64_feature_detection)";

    let stable_status = match name {
        "asimd" | "neon" => Some(std::arch::is_aarch64_feature_detected!("neon")),
        "aes" => Some(std::arch::is_aarch64_feature_detected!("aes")),
        "bf16" => Some(std::arch::is_aarch64_feature_detected!("bf16")),
        "bti" => Some(std::arch::is_aarch64_feature_detected!("bti")),
        "crc" => Some(std::arch::is_aarch64_feature_detected!("crc")),
        "dit" => Some(std::arch::is_aarch64_feature_detected!("dit")),
        "dotprod" => Some(std::arch::is_aarch64_feature_detected!("dotprod")),
        "dpb" => Some(std::arch::is_aarch64_feature_detected!("dpb")),
        "dpb2" => Some(std::arch::is_aarch64_feature_detected!("dpb2")),
        "f32mm" => Some(std::arch::is_aarch64_feature_detected!("f32mm")),
        "f64mm" => Some(std::arch::is_aarch64_feature_detected!("f64mm")),
        "fcma" => Some(std::arch::is_aarch64_feature_detected!("fcma")),
        "fhm" => Some(std::arch::is_aarch64_feature_detected!("fhm")),
        "flagm" => Some(std::arch::is_aarch64_feature_detected!("flagm")),
        "fp" => Some(std::arch::is_aarch64_feature_detected!("fp")),
        "fp16" => Some(std::arch::is_aarch64_feature_detected!("fp16")),
        "frintts" => Some(std::arch::is_aarch64_feature_detected!("frintts")),
        "i8mm" => Some(std::arch::is_aarch64_feature_detected!("i8mm")),
        "jsconv" => Some(std::arch::is_aarch64_feature_detected!("jsconv")),
        "lse" => Some(std::arch::is_aarch64_feature_detected!("lse")),
        "lse2" => Some(std::arch::is_aarch64_feature_detected!("lse2")),
        "mte" => Some(std::arch::is_aarch64_feature_detected!("mte")),
        "paca" => Some(std::arch::is_aarch64_feature_detected!("paca")),
        "pacg" => Some(std::arch::is_aarch64_feature_detected!("pacg")),
        "pmull" => Some(std::arch::is_aarch64_feature_detected!("pmull")),
        "rand" => Some(std::arch::is_aarch64_feature_detected!("rand")),
        "rcpc" => Some(std::arch::is_aarch64_feature_detected!("rcpc")),
        "rcpc2" => Some(std::arch::is_aarch64_feature_detected!("rcpc2")),
        "rdm" => Some(std::arch::is_aarch64_feature_detected!("rdm")),
        "sb" => Some(std::arch::is_aarch64_feature_detected!("sb")),
        "sha2" => Some(std::arch::is_aarch64_feature_detected!("sha2")),
        "sha3" => Some(std::arch::is_aarch64_feature_detected!("sha3")),
        "sm4" => Some(std::arch::is_aarch64_feature_detected!("sm4")),
        "ssbs" => Some(std::arch::is_aarch64_feature_detected!("ssbs")),
        "sve" => Some(std::arch::is_aarch64_feature_detected!("sve")),
        "sve2" => Some(std::arch::is_aarch64_feature_detected!("sve2")),
        "sve2-aes" => Some(std::arch::is_aarch64_feature_detected!("sve2-aes")),
        "sve2-bitperm" => Some(std::arch::is_aarch64_feature_detected!("sve2-bitperm")),
        "sve2-sha3" => Some(std::arch::is_aarch64_feature_detected!("sve2-sha3")),
        "sve2-sm4" => Some(std::arch::is_aarch64_feature_detected!("sve2-sm4")),
        "tme" => Some(std::arch::is_aarch64_feature_detected!("tme")),
        _ => None,
    };

    if let Some(present) = stable_status {
        return Available(present);
    }

    if let Some(present) = detect_unstable_aarch64_feature(name, is_nightly) {
        return Available(present);
    }

    Unsupported(NIGHTLY_MSG)
}

#[cfg(target_arch = "aarch64")]
fn detect_unstable_aarch64_feature(name: &str, is_nightly: bool) -> Option<bool> {
    if !is_nightly {
        return None;
    }

    detect_unstable_aarch64_feature_impl(name)
}

#[cfg(all(target_arch = "aarch64", feature = "nightly"))]
fn detect_unstable_aarch64_feature_impl(name: &str) -> Option<bool> {
    Some(match name {
        "cssc" => std::arch::is_aarch64_feature_detected!("cssc"),
        "ecv" => std::arch::is_aarch64_feature_detected!("ecv"),
        "faminmax" => std::arch::is_aarch64_feature_detected!("faminmax"),
        "flagm2" => std::arch::is_aarch64_feature_detected!("flagm2"),
        "fp8" => std::arch::is_aarch64_feature_detected!("fp8"),
        "fp8dot2" => std::arch::is_aarch64_feature_detected!("fp8dot2"),
        "fp8dot4" => std::arch::is_aarch64_feature_detected!("fp8dot4"),
        "fp8fma" => std::arch::is_aarch64_feature_detected!("fp8fma"),
        "fpmr" => std::arch::is_aarch64_feature_detected!("fpmr"),
        "hbc" => std::arch::is_aarch64_feature_detected!("hbc"),
        "lse128" => std::arch::is_aarch64_feature_detected!("lse128"),
        "lut" => std::arch::is_aarch64_feature_detected!("lut"),
        "mops" => std::arch::is_aarch64_feature_detected!("mops"),
        "pauth-lr" => std::arch::is_aarch64_feature_detected!("pauth-lr"),
        "rcpc3" => std::arch::is_aarch64_feature_detected!("rcpc3"),
        "sme" => std::arch::is_aarch64_feature_detected!("sme"),
        "sme-b16b16" => std::arch::is_aarch64_feature_detected!("sme-b16b16"),
        "sme-f16f16" => std::arch::is_aarch64_feature_detected!("sme-f16f16"),
        "sme-f64f64" => std::arch::is_aarch64_feature_detected!("sme-f64f64"),
        "sme-f8f16" => std::arch::is_aarch64_feature_detected!("sme-f8f16"),
        "sme-f8f32" => std::arch::is_aarch64_feature_detected!("sme-f8f32"),
        "sme-fa64" => std::arch::is_aarch64_feature_detected!("sme-fa64"),
        "sme-i16i64" => std::arch::is_aarch64_feature_detected!("sme-i16i64"),
        "sme-lutv2" => std::arch::is_aarch64_feature_detected!("sme-lutv2"),
        "sme2" => std::arch::is_aarch64_feature_detected!("sme2"),
        "sme2p1" => std::arch::is_aarch64_feature_detected!("sme2p1"),
        "ssve-fp8dot2" => std::arch::is_aarch64_feature_detected!("ssve-fp8dot2"),
        "ssve-fp8dot4" => std::arch::is_aarch64_feature_detected!("ssve-fp8dot4"),
        "ssve-fp8fma" => std::arch::is_aarch64_feature_detected!("ssve-fp8fma"),
        "sve-b16b16" => std::arch::is_aarch64_feature_detected!("sve-b16b16"),
        "sve2p1" => std::arch::is_aarch64_feature_detected!("sve2p1"),
        "wfxt" => std::arch::is_aarch64_feature_detected!("wfxt"),
        _ => return None,
    })
}

#[cfg(all(target_arch = "aarch64", not(feature = "nightly")))]
fn detect_unstable_aarch64_feature_impl(_name: &str) -> Option<bool> {
    None
}

#[cfg(target_arch = "aarch64")]
fn query_sve_vector_length_bits() -> Option<u64> {
    if std::arch::is_aarch64_feature_detected!("sve") {
        unsafe { Some(sve_cntb_bytes() * 8) }
    } else {
        None
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "sve")]
unsafe fn sve_cntb_bytes() -> u64 {
    let mut bytes: u64;
    unsafe {
        asm!("cntb {cnt}", cnt = out(reg) bytes, options(nomem, preserves_flags, nostack));
    }
    bytes
}
