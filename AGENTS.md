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
- Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` for behavior changes.
- Claude Code guidance lives in `CLAUDE.md`; keep both files consistent when changing repo workflows.
