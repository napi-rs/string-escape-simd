use std::time::Instant;
use string_escape_simd::{encode_str, encode_str_fallback};

fn main() {
    println!("V8-Style JSON Stringify Optimization Demo");
    println!("=========================================");
    
    // Test with the included fixture
    let fixture = include_str!("../cal.com.tsx");
    println!("Testing with cal.com.tsx fixture ({} bytes)", fixture.len());
    
    // Verify correctness
    let simd_result = encode_str(fixture);
    let fallback_result = encode_str_fallback(fixture);
    let serde_result = serde_json::to_string(fixture).unwrap();
    
    assert_eq!(simd_result, fallback_result, "SIMD and fallback results differ");
    assert_eq!(simd_result, serde_result, "Result doesn't match serde_json");
    println!("✓ Correctness verified - all implementations produce identical output");
    
    // Simple performance comparison (Note: May not show differences on x86_64)
    let iterations = 1000;
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encode_str_fallback(fixture);
    }
    let fallback_time = start.elapsed();
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = encode_str(fixture);
    }
    let simd_time = start.elapsed();
    
    println!("\nPerformance comparison ({} iterations):", iterations);
    println!("Fallback implementation: {:?}", fallback_time);
    println!("Optimized implementation: {:?}", simd_time);
    
    if simd_time < fallback_time {
        let improvement = (fallback_time.as_nanos() as f64 / simd_time.as_nanos() as f64) - 1.0;
        println!("Improvement: {:.1}% faster", improvement * 100.0);
    } else {
        println!("Note: Performance improvements are most visible on aarch64 architecture");
    }
    
    // Test with different string types
    println!("\nTesting different string patterns:");
    
    // Clean ASCII
    let clean_ascii = "Hello world! This is a clean ASCII string.".repeat(100);
    test_string_type("Clean ASCII", &clean_ascii);
    
    // With escapes
    let with_escapes = "Text with \"quotes\" and \\backslashes\\ and \nnewlines".repeat(50);
    test_string_type("With escapes", &with_escapes);
    
    // Mixed Unicode
    let mixed_unicode = "English text with 中文, emoji 🚀, and \"quotes\"".repeat(30);
    test_string_type("Mixed Unicode", &mixed_unicode);
    
    println!("\n✓ All tests completed successfully!");
}

fn test_string_type(name: &str, input: &str) {
    let result = encode_str(input);
    let expected = serde_json::to_string(input).unwrap();
    assert_eq!(result, expected, "Mismatch for {}", name);
    println!("  ✓ {}: {} bytes -> {} bytes", name, input.len(), result.len());
}