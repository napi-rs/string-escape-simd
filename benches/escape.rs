use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

use string_escape_simd::{encode_str, encode_str_fallback};

const FIXTURE: &str = include_str!("../cal.com.tsx");

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("escape simd", |b| b.iter(|| black_box(encode_str(FIXTURE))));
    c.bench_function("escape v_jsonescape", |b| {
        b.iter(|| black_box(v_jsonescape::escape(FIXTURE).to_string()))
    });
    c.bench_function("escape software", |b| {
        b.iter(|| black_box(encode_str_fallback(FIXTURE)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
