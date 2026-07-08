# list recipes
help:
    just --list

# format the code
fmt:
    cargo fmt

# alias for fmt
format: fmt

# check formatting without writing changes
fmt-check:
    cargo fmt -- --check

# lint the code without writing changes
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# run tests
test:
    cargo test

# check documentation with rustdoc warnings denied
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# verify package contents without publishing
package:
    cargo package

# build the crate
build:
    cargo build

# format, lint, test, document, and package like CI
check: fmt-check lint test doc-check package

# run the same checks and build mirrored by CI
ci: check build

# Cut a GitHub release for an explicit SemVer version.
cut-release *args:
    ./scripts/cut-release.sh {{args}}
