# Check formatting, lints, and tests.
check:
    cargo fmt -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test

# Cut a GitHub release.
cut-release *args:
    ./scripts/cut-release.sh {{args}}
