use std::{fs, hint::black_box};

use criterion::{Criterion, criterion_group, criterion_main};

use json_escape_simd::{escape, escape_generic};

fn get_rxjs_sources() -> Vec<String> {
    let rxjs_paths = glob::glob("node_modules/rxjs/src/**/*.ts").unwrap();
    let mut sources = Vec::new();
    for entry in rxjs_paths {
        let p = entry.unwrap();
        if fs::metadata(&p).unwrap().is_file() {
            sources.push(fs::read_to_string(&p).unwrap());
        }
    }
    sources
}

fn get_fixture_sources() -> Vec<String> {
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
        if fs::metadata(&p).unwrap().is_file() {
            sources.push(fs::read_to_string(&p).unwrap());
        }
    }
    sources
}

fn run_benchmarks(c: &mut Criterion, sources: &[String], prefix: &str) {
    c.bench_function(&format!("{} escape simd", prefix), |b| {
        b.iter(|| {
            for source in sources {
                black_box(escape(source));
            }
        })
    });
    #[cfg(not(feature = "codspeed"))]
    c.bench_function(&format!("{} escape v_jsonescape", prefix), |b| {
        b.iter(|| {
            for source in sources {
                black_box(v_jsonescape::escape(source).to_string());
            }
        })
    });
    #[cfg(not(feature = "codspeed"))]
    c.bench_function(&format!("{} json-escape", prefix), |b| {
        b.iter(|| {
            for source in sources {
                black_box(json_escape::escape_str(source).collect::<String>());
            }
        })
    });
    c.bench_function(&format!("{} escape generic", prefix), |b| {
        b.iter(|| {
            for source in sources {
                black_box(escape_generic(source));
            }
        })
    });
    #[cfg(not(feature = "codspeed"))]
    c.bench_function(&format!("{} serde_json", prefix), |b| {
        b.iter(|| {
            for source in sources {
                black_box(serde_json::to_string(source).unwrap());
            }
        })
    });
}

fn rxjs_benchmark(c: &mut Criterion) {
    let sources = get_rxjs_sources();
    if !sources.is_empty() {
        run_benchmarks(c, &sources, "rxjs");
    }
}

fn fixtures_benchmark(c: &mut Criterion) {
    let sources = get_fixture_sources();
    if !sources.is_empty() {
        run_benchmarks(c, &sources, "fixtures");
    }
}

criterion_group!(benches, rxjs_benchmark, fixtures_benchmark);
criterion_main!(benches);
