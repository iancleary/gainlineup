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

# check documentation with rustdoc warnings denied
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# verify the crate can be packaged without publishing
package:
    cargo package

# format-check, lint, test, document, and package
check: fmt-check lint test doc-check package

# run contributor checks and build
ci: check build
