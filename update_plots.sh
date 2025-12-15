#!/bin/bash

# Exit on error
set -e

echo "Updating plots for all top-level TOML configurations..."

# Run gainlineup for each configuration file
cargo run -- files/defaults_to_cw.toml
cargo run -- files/wideband.toml
cargo run -- files/compression/compression_test.toml
cargo run -- files/include_directive/include.toml
cargo run -- files/touchstone_options/config.toml

echo "All plots updated successfully."
