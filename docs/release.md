# Release Process

This repo has a deterministic local release runner at `scripts/cut-release.sh`.
Use `just cut-release` as the normal entrypoint.

## Versioning

The crate version is SemVer and lives in the root `Cargo.toml`. The next version
is not inferred because the repo has no checked-in bump policy. Pass the intended
version explicitly with `--version`. The runner accepts a version that differs
from the manifest version and also supports releasing the already-set manifest
version when its tag is unused.

Read-only queries:

```bash
just cut-release --print-current-version
just cut-release --print-next-version --version 0.23.1
```

## Dry Run

Dry-run mode must not mutate public state:

```bash
just cut-release --dry-run --version 0.23.1 --notes-file /tmp/gainlineup-notes.md
```

The dry run operates on a temporary archive of `HEAD` and does not edit files,
create commits, create tags, push, or create a GitHub release. When the target
version differs from the manifest version, it validates that version update.
When the target matches the manifest version, it validates a release of the
current checkout without changing version files or creating a commit. The tag
must not already exist in either case.

## Real Release

Prepare release notes in a local markdown file, then run from the default branch
with a clean working tree:

```bash
just cut-release --version 0.23.1 --notes-file /tmp/gainlineup-notes.md
```

If the target version differs from the manifest version, the runner updates
`Cargo.toml` and `Cargo.lock` and commits those changes. If the target matches
the manifest version, it leaves both files unchanged and tags the verified
current `HEAD`. Both paths require the default branch, a clean working tree, and
an unused version tag. The runner runs:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

After checks pass, a changed version receives an annotated `v<version>` tag on
the release commit. An already-bumped version receives that tag on the current
`HEAD`. The runner pushes the branch and tag, then creates the GitHub release
with `gh release create`.

Publishing to crates.io is handled by GitHub Actions: the published GitHub
release event runs CI and then `cargo publish --verbose` with
`CARGO_REGISTRY_TOKEN`.

When a version has already been committed to the default branch, release that
exact manifest version without another version-only commit:

```bash
just cut-release --dry-run --version 0.23.0 --notes-file /tmp/gainlineup-0.23.0-notes.md
just cut-release --version 0.23.0 --notes-file /tmp/gainlineup-0.23.0-notes.md
```

This path requires the same clean default-branch checkout and unused tag as a
version-bumping release. It runs the same local checks, tags current `HEAD`,
pushes the tag, and creates the GitHub release.

Run the isolated release-runner regression proof with:

```bash
bash scripts/test-cut-release.sh
```

## Agent Routing

Use `create-release-process` when maintaining this workflow. Use `cut-release`
when executing an ordinary release request through the checked-in runner.
