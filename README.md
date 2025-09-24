# json-escape-simd

![Crates.io Version](https://img.shields.io/crates/v/json-escape-simd)
![docs.rs](https://img.shields.io/docsrs/json-escape-simd)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/napi-rs/json-escape-simd)

Optimized SIMD routines for escaping JSON strings. This repository contains the `json-escape-simd` crate, comparison fixtures, and Criterion benches against commonly used alternatives.

> [!IMPORTANT]
>
> On aarch64 NEON hosts the available register width is **128** bits, which is narrower than the lookup table this implementation prefers. As a result the SIMD path may not outperform the generic fallback, which is reflected in the benchmark numbers below.
>
> On some modern macOS devices with larger register numbers, the SIMD path may outperform the generic fallback, see the [M3 max benchmark](#apple-m3-max) below.

> [!NOTE]
>
> The `force_aarch64_neon` feature flag can be used to force use of the neon implementation on aarch64. This is useful for the benchmark.

## Benchmarks

Numbers below come from `cargo bench` runs on GitHub Actions hardware. Criterion reports are summarized to make it easier to spot relative performance. "vs fastest" shows how much slower each implementation is compared to the fastest entry in the table (1.00× means fastest).

### GitHub Actions x86_64 (`ubuntu-latest`)

`AVX2` enabled.

**RxJS payload (~10k iterations)**

| Implementation        | Median time   | vs fastest |
| --------------------- | ------------- | ---------- |
| **`escape simd`**     | **341.18 µs** | **1.00×**  |
| `escape v_jsonescape` | 555.47 µs     | 1.63×      |
| `escape generic`      | 656.85 µs     | 1.93×      |
| `serde_json`          | 744.75 µs     | 2.18×      |
| `json-escape`         | 777.15 µs     | 2.28×      |

**Fixtures payload (~300 iterations)**

| Implementation        | Median time  | vs fastest |
| --------------------- | ------------ | ---------- |
| **`escape simd`**     | **12.67 ms** | **1.00×**  |
| `escape v_jsonescape` | 20.58 ms     | 1.62×      |
| `escape generic`      | 22.57 ms     | 1.78×      |
| `serde_json`          | 24.52 ms     | 1.94×      |
| `json-escape`         | 26.97 ms     | 2.13×      |

### GitHub Actions aarch64 (`ubuntu-24.04-arm`)

Neon enabled.

**RxJS payload (~10k iterations)**

| Implementation        | Median time   | vs fastest |
| --------------------- | ------------- | ---------- |
| **`escape generic`**  | **546.89 µs** | **1.00×**  |
| `escape simd`         | 589.29 µs     | 1.08×      |
| `serde_json`          | 612.33 µs     | 1.12×      |
| `json-escape`         | 624.66 µs     | 1.14×      |
| `escape v_jsonescape` | 789.14 µs     | 1.44×      |

**Fixtures payload (~300 iterations)**

| Implementation        | Median time  | vs fastest |
| --------------------- | ------------ | ---------- |
| **`escape generic`**  | **17.81 ms** | **1.00×**  |
| `serde_json`          | 19.77 ms     | 1.11×      |
| `json-escape`         | 20.84 ms     | 1.17×      |
| `escape simd`         | 21.04 ms     | 1.18×      |
| `escape v_jsonescape` | 25.57 ms     | 1.44×      |

### GitHub Actions macOS (`macos-latest`)

> Apple M1 chip

**RxJS payload (~10k iterations)**

| Implementation        | Median time   | vs fastest |
| --------------------- | ------------- | ---------- |
| **`escape generic`**  | **759.07 µs** | **1.00×**  |
| `escape simd`         | 764.98 µs     | 1.01×      |
| `serde_json`          | 793.91 µs     | 1.05×      |
| `json-escape`         | 868.21 µs     | 1.14×      |
| `escape v_jsonescape` | 926.00 µs     | 1.22×      |

**Fixtures payload (~300 iterations)**

| Implementation        | Median time  | vs fastest |
| --------------------- | ------------ | ---------- |
| **`serde_json`**      | **26.41 ms** | **1.00×**  |
| `escape generic`      | 26.43 ms     | 1.00×      |
| `escape simd`         | 26.42 ms     | 1.00×      |
| `json-escape`         | 28.94 ms     | 1.10×      |
| `escape v_jsonescape` | 29.22 ms     | 1.11×      |

### Apple M3 Max

**RxJS payload (~10k iterations)**

| Implementation        | Median time   | vs fastest |
| --------------------- | ------------- | ---------- |
| **`escape simd`**     | **307.20 µs** | **1.00×**  |
| `escape generic`      | 490.00 µs     | 1.60×      |
| `serde_json`          | 570.35 µs     | 1.86×      |
| `escape v_jsonescape` | 599.72 µs     | 1.95×      |
| `json-escape`         | 644.73 µs     | 2.10×      |

**Fixtures payload (~300 iterations)**

| Implementation        | Median time  | vs fastest |
| --------------------- | ------------ | ---------- |
| **`escape generic`**  | **17.89 ms** | **1.00×**  |
| **`escape simd`**     | **17.92 ms** | **1.00×**  |
| `serde_json`          | 19.78 ms     | 1.11×      |
| `escape v_jsonescape` | 21.09 ms     | 1.18×      |
| `json-escape`         | 22.43 ms     | 1.25×      |
