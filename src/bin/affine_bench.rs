use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use string_escape_simd::{encode_str, encode_str_fallback};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <mode> [options]", args[0]);
        eprintln!("Modes:");
        eprintln!("  simd           - Benchmark optimized SIMD implementation");
        eprintln!("  fallback       - Benchmark fallback implementation");
        eprintln!("  compare        - Compare both implementations");
        eprintln!("  individual     - Process individual files from AFFiNE");
        eprintln!("  hyperfine      - Silent mode for hyperfine benchmarking");
        std::process::exit(1);
    }

    let mode = &args[1];
    
    // Load the AFFiNE dataset
    let benchmark_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmark_data");
    let all_files_path = benchmark_data_dir.join("all_files.js");
    let file_list_path = benchmark_data_dir.join("file_list.txt");
    
    if !all_files_path.exists() {
        eprintln!("Error: AFFiNE benchmark data not found at {:?}", all_files_path);
        eprintln!("Please run the data collection script first.");
        std::process::exit(1);
    }

    match mode.as_str() {
        "simd" => bench_simd(&all_files_path),
        "fallback" => bench_fallback(&all_files_path),
        "compare" => compare_implementations(&all_files_path),
        "individual" => bench_individual_files(&file_list_path),
        "hyperfine" => hyperfine_mode(&all_files_path),
        _ => {
            eprintln!("Unknown mode: {}. Use 'simd', 'fallback', 'compare', 'individual', or 'hyperfine'", mode);
            std::process::exit(1);
        }
    }
}

fn bench_simd(data_path: &Path) {
    let content = fs::read_to_string(data_path)
        .expect("Failed to read benchmark data");
    
    println!("Benchmarking SIMD implementation with AFFiNE dataset");
    println!("Dataset size: {} bytes ({:.1} MB)", content.len(), content.len() as f64 / 1_000_000.0);
    
    let iterations = 10;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _result = encode_str(&content);
    }
    
    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;
    let throughput = (content.len() as f64 / per_iteration.as_secs_f64()) / 1_000_000.0;
    
    println!("SIMD implementation:");
    println!("  Total time: {:?} ({} iterations)", elapsed, iterations);
    println!("  Per iteration: {:?}", per_iteration);
    println!("  Throughput: {:.1} MB/s", throughput);
}

fn bench_fallback(data_path: &Path) {
    let content = fs::read_to_string(data_path)
        .expect("Failed to read benchmark data");
    
    println!("Benchmarking fallback implementation with AFFiNE dataset");
    println!("Dataset size: {} bytes ({:.1} MB)", content.len(), content.len() as f64 / 1_000_000.0);
    
    let iterations = 10;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _result = encode_str_fallback(&content);
    }
    
    let elapsed = start.elapsed();
    let per_iteration = elapsed / iterations;
    let throughput = (content.len() as f64 / per_iteration.as_secs_f64()) / 1_000_000.0;
    
    println!("Fallback implementation:");
    println!("  Total time: {:?} ({} iterations)", elapsed, iterations);
    println!("  Per iteration: {:?}", per_iteration);
    println!("  Throughput: {:.1} MB/s", throughput);
}

fn compare_implementations(data_path: &Path) {
    let content = fs::read_to_string(data_path)
        .expect("Failed to read benchmark data");
    
    println!("Comparing implementations with AFFiNE dataset");
    println!("Dataset size: {} bytes ({:.1} MB)", content.len(), content.len() as f64 / 1_000_000.0);
    
    // Verify correctness first
    let simd_result = encode_str(&content);
    let fallback_result = encode_str_fallback(&content);
    
    if simd_result != fallback_result {
        eprintln!("Error: SIMD and fallback implementations produce different results!");
        std::process::exit(1);
    }
    
    println!("✓ Correctness verified - both implementations produce identical output");
    println!("  Output size: {} bytes ({:.1} MB)", simd_result.len(), simd_result.len() as f64 / 1_000_000.0);
    
    let iterations = 10;
    
    // Benchmark fallback
    let start = Instant::now();
    for _ in 0..iterations {
        let _result = encode_str_fallback(&content);
    }
    let fallback_time = start.elapsed();
    
    // Benchmark SIMD
    let start = Instant::now();
    for _ in 0..iterations {
        let _result = encode_str(&content);
    }
    let simd_time = start.elapsed();
    
    let fallback_per_iter = fallback_time / iterations;
    let simd_per_iter = simd_time / iterations;
    let fallback_throughput = (content.len() as f64 / fallback_per_iter.as_secs_f64()) / 1_000_000.0;
    let simd_throughput = (content.len() as f64 / simd_per_iter.as_secs_f64()) / 1_000_000.0;
    
    println!("\nPerformance comparison ({} iterations):", iterations);
    println!("Fallback implementation:");
    println!("  Per iteration: {:?}", fallback_per_iter);
    println!("  Throughput: {:.1} MB/s", fallback_throughput);
    
    println!("SIMD implementation:");
    println!("  Per iteration: {:?}", simd_per_iter);
    println!("  Throughput: {:.1} MB/s", simd_throughput);
    
    if simd_time < fallback_time {
        let improvement = (fallback_time.as_nanos() as f64 / simd_time.as_nanos() as f64) - 1.0;
        println!("\n🚀 SIMD is {:.1}% faster", improvement * 100.0);
        println!("   Speedup: {:.2}x", fallback_time.as_secs_f64() / simd_time.as_secs_f64());
    } else if fallback_time < simd_time {
        let regression = (simd_time.as_nanos() as f64 / fallback_time.as_nanos() as f64) - 1.0;
        println!("\n⚠️  SIMD is {:.1}% slower (expected on non-aarch64)", regression * 100.0);
    } else {
        println!("\n📊 Performance is equivalent");
    }
}

fn bench_individual_files(file_list_path: &Path) {
    let file_list = fs::read_to_string(file_list_path)
        .expect("Failed to read file list");
    
    let affine_root = "/tmp/affine/AFFiNE-0.23.2";
    let files: Vec<_> = file_list
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    
    println!("Benchmarking individual files from AFFiNE dataset");
    println!("Processing {} files", files.len());
    
    let mut total_bytes = 0;
    let mut total_simd_time = std::time::Duration::ZERO;
    let mut total_fallback_time = std::time::Duration::ZERO;
    let mut processed_files = 0;
    
    for (i, file_path) in files.iter().enumerate() {
        let full_path = Path::new(affine_root).join(file_path.trim_start_matches("./"));
        
        if !full_path.exists() || !full_path.is_file() {
            continue;
        }
        
        if let Ok(content) = fs::read_to_string(&full_path) {
            total_bytes += content.len();
            
            // Benchmark fallback
            let start = Instant::now();
            let _fallback_result = encode_str_fallback(&content);
            total_fallback_time += start.elapsed();
            
            // Benchmark SIMD
            let start = Instant::now();
            let _simd_result = encode_str(&content);
            total_simd_time += start.elapsed();
            
            processed_files += 1;
            
            if (i + 1) % 1000 == 0 {
                println!("Processed {}/{} files...", i + 1, files.len());
            }
        }
    }
    
    println!("\nIndividual files benchmark results:");
    println!("  Processed files: {}", processed_files);
    println!("  Total size: {} bytes ({:.1} MB)", total_bytes, total_bytes as f64 / 1_000_000.0);
    println!("  Fallback total time: {:?}", total_fallback_time);
    println!("  SIMD total time: {:?}", total_simd_time);
    
    if total_simd_time < total_fallback_time {
        let improvement = (total_fallback_time.as_nanos() as f64 / total_simd_time.as_nanos() as f64) - 1.0;
        println!("  🚀 SIMD is {:.1}% faster overall", improvement * 100.0);
    }
}

fn hyperfine_mode(data_path: &Path) {
    let content = fs::read_to_string(data_path)
        .expect("Failed to read benchmark data");
    
    // For hyperfine, we want to be silent and just do the work
    // The specific implementation is chosen via arguments
    let args: Vec<String> = env::args().collect();
    let default_impl = "simd".to_string();
    let implementation = args.get(2).unwrap_or(&default_impl);
    
    match implementation.as_str() {
        "simd" => {
            let _result = encode_str(&content);
        }
        "fallback" => {
            let _result = encode_str_fallback(&content);
        }
        _ => {
            // Default to SIMD
            let _result = encode_str(&content);
        }
    }
}