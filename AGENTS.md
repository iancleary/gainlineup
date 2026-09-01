# AGENTS.md - gainlineup

Rust crate for RF signal chain cascade analysis: gain, noise figure, P1dB,
IP3/IMD3, dynamic range, and AM-AM/AM-PM modeling.

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
- Repo operation belongs here and in `CLAUDE.md`.
- Release procedure belongs in `docs/release.md` and the release runner.
- Regression knowledge belongs in focused tests, especially README-mirrored tests
  for public examples and `tests/integration_rf_scenarios.rs` for realistic
  chains.

Do not add broad planning files unless they will be maintained by an existing
workflow. Prefer tightening the nearest doc, test name, or example over adding a
new surface.

## Commands

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
cargo run -- files/wideband.toml
cargo doc --open
just cut-release --dry-run --version <semver> --notes-file <path>
```

## Releases

Maintain the deterministic release workflow with `create-release-process`.
Execute ordinary releases with `cut-release` via `just cut-release`; see
`docs/release.md` for the repo-local contract. The runner requires an explicit
SemVer `--version`, supports read-only version queries, and creates the GitHub
release, which triggers the crates.io publish workflow.

## Notes

- Keep changes minimal and aligned to the crate's RF modeling purpose.
- Run `git diff --check` for every change.
- Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` for behavior changes.
- Run `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps` when changing public rustdoc, README examples, or exported APIs.
- Claude Code guidance lives in `CLAUDE.md`; keep both files consistent when changing repo workflows.
