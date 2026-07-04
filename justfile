# kou — virtual terminal automation.

set shell := ["bash", "-c"]

default:
    @just --list

# Format all sources.
fmt:
    cargo fmt --all

# Check formatting without writing.
fmt-check:
    cargo fmt --all -- --check

# Type-check all targets and features.
check:
    KOU_SKIP_FONT_FETCH=1 cargo check --all-targets --all-features

# Clippy with -D warnings.
clippy:
    KOU_SKIP_FONT_FETCH=1 cargo clippy --all-targets --all-features -- -D warnings

# Run the test suite.
test:
    KOU_SKIP_FONT_FETCH=1 cargo test --all-features

# Build all features.
build:
    KOU_SKIP_FONT_FETCH=1 cargo build --all-features

# One-shot local gate: fmt-check + clippy + test.
ci:
    just fmt-check
    just clippy
    just test
