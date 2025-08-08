// Legacy criterion benchmark - superseded by real-world AFFiNE benchmark
// Use `./benchmark.sh` or `cargo run --bin affine_bench` for comprehensive testing

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

use string_escape_simd::{encode_str, encode_str_fallback};

const FIXTURE: &str = include_str!("../cal.com.tsx");

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("escape simd", |b| b.iter(|| encode_str(black_box(FIXTURE))));
    c.bench_function("escape software", |b| {
        b.iter(|| encode_str_fallback(black_box(FIXTURE)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
