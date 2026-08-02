# CLAUDE.md — gainlineup

## Overview

Rust crate for RF signal chain (gain lineup) cascade analysis. Models amplifiers, filters, attenuators, and mixers — cascading gain, noise figure (Friis equation), P1dB compression, IP3/IMD3, and dynamic range. Published on crates.io (v0.22.2).

## Agent Usage

Use `gainlineup` when the task describes an ordered RF chain: LNAs, filters,
attenuators, mixers, power amplifiers, gain compression, cascaded noise figure,
OIP3/IIP3, SFDR, dynamic range, or AM-AM/AM-PM behavior. Model each hardware
stage as a `Block`, create one `Input`, then call
`cascade_vector_return_vector` when intermediate stage outputs matter or
`cascade_vector_return_output` when only the final result matters.

Do not use this crate for file-based S-parameter network analysis; use
`touchstone` for `.sNp` parsing and matrix/network-parameter work. Do not use it
for full radio-link BER/margin/orbit/Doppler questions; use `linkbudget` there.
Use `rfconversions` for standalone scalar conversions before building a chain.

Keep the RF semantics straight: `gain_db` may be negative for losses, passive
losses should usually have matching positive `noise_figure_db`, P1dB and IP3
fields are output-referred dBm values, and `Input::noise_temperature_k` is an
optional source/system temperature contribution rather than a block NF.

## Commands

```bash
cargo test                        # Run all 96 tests (v0.22.2)
cargo clippy -- -D warnings       # Lint
cargo fmt -- --check              # Format check
just cut-release --dry-run --version <semver> --notes-file <path>  # Preview release
cargo run -- files/wideband.toml  # CLI: cascade from TOML, generates HTML
cargo doc --open                  # Generate and view API docs
```

## Releases

Maintain the deterministic release workflow with `create-release-process`.
Execute ordinary releases with `cut-release` via `just cut-release`; see
`docs/release.md` for the repo-local contract. The runner requires an explicit
SemVer `--version`, supports read-only version queries, and creates the GitHub
release, which triggers the crates.io publish workflow.

## Module Map

| Module | File | Description |
|--------|------|-------------|
| `block` | `src/block.rs` | `Block` struct — gain, NF, P1dB, IP3; AM-AM sweeps, IMD3 |
| `input` | `src/input.rs` | `Input` struct — signal power, frequency, bandwidth, noise temp |
| `node` | `src/node.rs` | `SignalNode` — cascade result at each stage; `DynamicRange` summary |
| `amplifier_model` | `src/amplifier_model.rs` | `AmplifierModel` — wraps Block with AM-PM characterization |
| `constants` | `src/constants.rs` | Physical constants (kB, T0) |
| `cli` | `src/cli.rs` | CLI: reads TOML, runs cascade, generates HTML output |
| `file_operations` | `src/file_operations.rs` | File I/O utilities |
| `open` | `src/open.rs` | Cross-platform file/URL opening |
| `plot` | `src/plot.rs` | HTML table/plot generation (behind `plot` feature) |

## Key Public Functions

- `cascade_vector_return_output(input, blocks)` → final `SignalNode`
- `cascade_vector_return_vector(input, blocks)` → `Vec<SignalNode>` at every stage
- `cascade_am_am_sweep(blocks, start, stop, step)` → Pin vs Pout curve
- `cascade_gain_compression_sweep(blocks, start, stop, step)` → Pin vs Gain curve

## Where to Look

- **README.md** — Comprehensive examples: cascade, compression, AM-AM, IMD3, dynamic range, AM-PM, CLI TOML format
- **src/lib.rs** — Public API surface and cascade functions
- **src/block.rs** — Core Block type with compression, IP3, sweep methods
- **src/node.rs** — SignalNode with cascaded metrics and dynamic range summary
- **src/amplifier_model.rs** — AM-PM modeling, EVM from distortion, backoff calculations
- **files/** — Example TOML input files for CLI
- Tests are co-located in each module file
