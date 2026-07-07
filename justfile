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

# lint the code
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# run tests
test:
    cargo test

# build the crate
build:
    cargo build

# check documentation
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# verify the crate can be packaged without publishing
package:
    cargo publish --dry-run

# format-check, lint, test, document, and package
check: fmt-check lint test doc package

# run contributor checks and build
ci: check build
