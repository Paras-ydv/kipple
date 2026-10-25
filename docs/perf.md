# Performance

Benchmark: `cargo bench -p kipple-core --bench walk -- [entries] [runs]`. It generates a home-like tree (projects with sources, assets and `node_modules`) and runs the scan path with each walker. One warm-up run, then measured runs; the OS file cache is warm, so these are warm-cache numbers.

- **scan**: walk + artifact matching (named rules, sibling checks, the two generic `*` rules) + sizing of the 2,000 matched `node_modules`.
- **walk**: no rules, so every entry is traversed.

## 2026-10-26 — walker choice (SPEC §22 Q1)

Apple M4, 10 cores, 16 GB, APFS on internal SSD, macOS 27.0.1, Rust 1.99.1, release profile. 532,000 generated entries, 7 measured runs.

| Mode | Walker | p50 | p90 | Entries walked |
|---|---|---:|---:|---:|
| scan | `ignore` 0.4.33 | 2,588 ms | 2,686 ms | 96,041 |
| scan | `jwalk` 0.9.0 | 2,644 ms | 2,701 ms | 96,041 |
| walk | `ignore` 0.4.33 | 3,782 ms | 3,899 ms | 536,041 |
| walk | `jwalk` 0.9.0 | 3,804 ms | 3,840 ms | 536,041 |

An earlier 5-run pass had jwalk 4% ahead on scan; the order flips between runs. The two are within noise. See decision 0007.

Extrapolated, a 1M-entry home completes in well under the 30 s target of SPEC §5.3 (warm cache). Cold-cache numbers on a real home are still to be recorded.

## Known limitations

- APFS clones can't be detected cheaply; `on_disk` is reported as the upper bound (SPEC §5.5).
- Windows `on_disk` is the length rounded up to 4 KiB until `GetCompressedFileSizeW` lands (v1).
