# Cascade performance: 0.23.1

This pass removes repeated allocations and copies from scalar cascades and
frequency sweeps. A lineup made entirely of constant blocks calculates its RF
metrics once and copies the results with each requested frequency. Mixed and
tabulated lineups still evaluate every frequency. RF equations, interpolation,
floating-point operation order, and public result types are unchanged.

## Results

Each value below is the median of three process-run medians. Each process run
measured seven samples per workload. Times include creation and destruction of
the returned results. Speedup is baseline time divided by optimized time.

| Workload | Baseline (µs/op) | Optimized (µs/op) | Speedup | Time reduction |
|---|---:|---:|---:|---:|
| Scalar, all stage outputs | 1.0372 | 0.6561 | 1.58× | 36.7% |
| Scalar, final output only | 0.8116 | 0.6750 | 1.20× | 16.8% |
| Constant frequency sweep | 1138.7115 | 173.0188 | 6.58× | 84.8% |
| Mixed frequency sweep | 1272.1851 | 717.3135 | 1.77× | 43.6% |
| Dense tabulated sweep | 3302.2361 | 2228.2972 | 1.48× | 32.5% |

These are local measurements of synthetic workloads. They are not a performance
guarantee for other machines, lineups, or tracing configurations. The frequency
sweep timings exclude TOML/Touchstone parsing and CSV output.

## Workloads and environment

- Baseline: `v0.23.0`, commit `1e26e294f753750648d95b1a5dac45f8dda8433c`.
- Optimized implementation and benchmark: commit
  `21fca6d9cd7a7cdcb791cbd9e03326274d1ac82c`.
- Platform: ARM64 macOS 27.0.1, Rust 1.99.0 (`b940084d7`), LLVM 23.1.1.
- Build: Cargo's default bench/release optimization, default crate features,
  no tracing subscriber, and no extra compiler flags.
- Both scalar workloads use eight stages. The timed operation includes cloning
  the prebuilt block vector because the scalar API takes ownership.
- Every sweep uses 1,001 linearly spaced frequencies from 0.8 to 1.2 GHz. The
  constant sweep has eight stages. The mixed sweep has four constant stages and
  four tables with 65 samples each. The dense sweep has 16 tables with 257
  samples each.
- Input conditions are −18 dBm, 2 MHz bandwidth, and 285 K source temperature.
  Component tables and frequency grids are constructed before timing. Creating
  the scalar `Input` value is included in the timed operation.

The same benchmark source was compiled against each implementation. Each process
calibrated its batch size for approximately 120 ms per sample. The six processes
ran sequentially in this order: baseline, optimized, optimized, baseline,
baseline, optimized. No other builds or tests ran during measurement.

## Accuracy checks

All five node fingerprints matched across all six runs. Fingerprints encode each
node's name, frequency, bandwidth, power, noise, gain, NF, noise temperature,
OIP3, SFDR, and P1dB. Floats use their exact bits; optional values include their
presence tag. Fingerprints cover node fields, not the sweep container itself.

`tests/cascade_equivalence.rs` separately compares every node field and the
sweep's frequency labels bit for bit against explicit scalar chaining. It covers
single and multiple frequencies, constant and mixed lineups, exact table samples,
interpolation, compression, passive loss, missing intercepts, source temperatures,
and nonfinite-result rejection. These tests passed in debug and release builds.
The existing analytic RF tests also passed. This establishes agreement with the
existing model for the tested cases; it does not expand that model's accuracy.

## Repeat the measurements

```bash
just bench
PERF_FILTER=mixed-sweep just bench
cargo test --release --test cascade_equivalence
```

For a before/after comparison, use the same
[`cascade_performance.rs`](../benches/cascade_performance.rs) and its `[[bench]]`
manifest entry on both revisions. Version 0.23.0 predates that harness. Build both
binaries before timing, then run them sequentially on the same machine and Rust
toolchain. Compare node fingerprints as well as timings. Keep builds and tests
out of the measurement window.

## Recorded process medians

Values are nanoseconds per operation. Each cell contains the three process-run
medians for that implementation, in run order.

| Workload | Baseline runs (ns/op) | Optimized runs (ns/op) |
|---|---|---|
| Scalar, all stage outputs | 1018.1; 1037.2; 1039.5 | 656.1; 670.1; 652.8 |
| Scalar, final output only | 808.2; 811.6; 815.8 | 669.9; 675.0; 677.2 |
| Constant frequency sweep | 1136981.0; 1138711.5; 1156508.4 | 171591.8; 173018.8; 173288.9 |
| Mixed frequency sweep | 1264547.7; 1278120.6; 1272185.1 | 716940.6; 717313.5; 720551.6 |
| Dense tabulated sweep | 3316776.6; 3302236.1; 3276734.6 | 2228297.2; 2206669.7; 2229330.3 |
