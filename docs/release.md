# Release Process

This repo has a deterministic local release runner at `scripts/cut-release.sh`.
Use `just cut-release` as the normal entrypoint.

## Versioning

The crate version is SemVer and lives in the root `Cargo.toml`. The next version
is not inferred because the repo has no checked-in bump policy. Pass the intended
version explicitly with `--version`.

Read-only queries:

```bash
just cut-release --print-current-version
just cut-release --print-next-version --version 0.22.3
```

## Dry Run

Dry-run mode must not mutate public state:

```bash
just cut-release --dry-run --version 0.22.3 --notes-file /tmp/gainlineup-notes.md
```

The dry run operates on a temporary archive of `HEAD`, validates the version
bump, and does not edit files, create commits, create tags, push, or create a
GitHub release.

## Real Release

Prepare release notes in a local markdown file, then run from the default branch
with a clean working tree:

```bash
just cut-release --version 0.22.3 --notes-file /tmp/gainlineup-notes.md
```

The runner updates `Cargo.toml` and `Cargo.lock`, runs:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

It then commits `chore: release v<version>`, creates an annotated `v<version>`
tag, pushes the branch and tag, and finally creates the GitHub release with
`gh release create`. The runner does not publish to crates.io; handle any crate
publishing as a separate, explicit step.

## Agent Routing

Use `create-release-process` when maintaining this workflow. Use `cut-release`
when executing an ordinary release request through the checked-in runner.
