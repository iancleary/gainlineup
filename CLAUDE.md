# CLAUDE.md — gainlineup

## Overview

Rust crate for RF signal chain (gain lineup) cascade analysis. Models amplifiers, filters, attenuators, and mixers — cascading gain, noise figure (Friis equation), P1dB compression, IP3/IMD3, and dynamic range. Published on crates.io (v0.19.0).

## Commands

```bash
cargo test                        # Run all 96+ tests
cargo clippy -- -D warnings       # Lint
cargo fmt -- --check              # Format check
cargo run -- files/wideband.toml  # CLI: cascade from TOML, generates HTML
cargo doc --open                  # Generate and view API docs
```

## Module Map

| Module | File | Description |
|--------|------|-------------|
| `block` | `src/block.rs` | `Block` struct — gain, NF, P1dB, IP3; AM-AM sweeps, IMD3 |
| `input` | `src/input.rs` | `Input` struct — signal power, frequency, bandwidth, noise temp |
| `node` | `src/node.rs` | `SignalNode` — cascade result at each stage; `DynamicRange` summary |
| `amplifier_model` | `src/amplifier_model.rs` | `AmplifierModel` — wraps Block with AM-PM characterization |
| `constants` | `src/constants.rs` | Physical constants (kB, T0) |
| `cli` | `src/cli.rs` | CLI: reads TOML, runs cascade, generates HTML output |
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
