# AGENTS.md - gainlineup

Rust crate for RF signal chain cascade analysis: gain, noise figure, P1dB,
IP3/IMD3, dynamic range, and AM-AM/AM-PM modeling.

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
release as the final public step of a real release.

## Notes

- Keep changes minimal and aligned to the crate's RF modeling purpose.
- Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test` for behavior changes.
- Claude Code guidance lives in `CLAUDE.md`; keep both files consistent when changing repo workflows.
