#!/bin/bash

# Exit on error
set -e

echo "Updating plots for all top-level TOML configurations..."

# Run gainlineup for each configuration file
cargo run -- files/compression/compression_test.toml
cargo run -- files/include_directive/config.toml
cargo run -- files/simple_config.toml
cargo run -- files/simple_wideband_config.toml
cargo run -- files/touchstone_options/config.toml

echo "All plots updated successfully."
