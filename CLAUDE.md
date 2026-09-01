# CLAUDE.md — gainlineup

## Overview

Rust crate for RF signal chain (gain lineup) cascade analysis. Models amplifiers, filters, attenuators, and mixers — cascading gain, noise figure (Friis equation), P1dB compression, IP3/IMD3, and dynamic range. Published on crates.io; the current crate version lives in `Cargo.toml`.

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

## Operating Loop

Start with the artifact closest to the requested behavior:

- Public API behavior: read `src/lib.rs`, then `src/block.rs`, `src/input.rs`,
  `src/node.rs`, or `src/amplifier_model.rs`.
- CLI file behavior: read `src/cli.rs`, `src/plot.rs`, `src/file_operations.rs`,
  and the TOML examples under `files/`.
- Public examples: read `README.md` and the matching `tests/readme_*.rs` file.
- Release mechanics: read `docs/release.md`, `justfile`, and
  `scripts/cut-release.sh`.

Then make the smallest coherent change that preserves the RF model. Prefer
explicit units in names and examples (`*_db`, `*_dbm`, `*_hz`, `*_k`) and keep
README snippets mirrored by `tests/readme_*.rs` whenever public examples change.

## RF Model Invariants

- `Block` is a stage model with scalar gain, noise figure, optional output P1dB,
  and optional output IP3. Keep richer PA behavior in `AmplifierModel` unless a
  cascade-level API genuinely needs it.
- `Input` owns source signal power, center frequency, bandwidth, and optional
  source noise temperature. It is not a block and does not have gain or NF.
- `SignalNode` is the output of a stage. Its cumulative fields describe the
  chain up to that node, not just the latest block.
- Compression currently clamps power at `output_p1db_dbm + 1 dB`. If that model
  changes, update `Block::output_power`, node cascade behavior, README examples,
  and compression tests together.
- Cascaded NF and OIP3 calculations are order-sensitive. Add or update
  node-by-node tests for changes that alter ordering, gain signs, passive loss,
  source temperature, or intercept math.

## Agent Accretion

When you learn something durable, put it where the next agent will look first:

- API semantics belong in rustdoc and README examples.
- Repo operation belongs here and in `AGENTS.md`.
- Release procedure belongs in `docs/release.md` and the release runner.
- Regression knowledge belongs in focused tests, especially README-mirrored tests
  for public examples and `tests/integration_rf_scenarios.rs` for realistic
  chains.

Do not add broad planning files unless they will be maintained by an existing
workflow. Prefer tightening the nearest doc, test name, or example over adding a
new surface.

## Commands

```bash
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
just cut-release --dry-run --version <semver> --notes-file <path>
cargo run -- files/wideband.toml
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
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
