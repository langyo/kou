# kou — virtual terminal automation.

set shell := ["bash", "-c"]
# On Windows just resolves recipe shebangs through the shell named here; without
# it just falls back to `cygpath`, which Git for Windows does not put on PATH,
# so every shebang recipe fails with "could not find cygpath executable".
set windows-shell := ["bash.exe", "-c"]
# `set lists` enables which() (used by the imported celestia-devtools.just);
# `set unstable` gates it.
set unstable
set lists

import "./celestia-devtools.just"

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

# ── npx distribution (local dry-run) ─────────────────────────────────────────
#
# Wraps the shared recipe from celestia-devtools.just with kou's metadata. CI
# does the actual publish (see .github/workflows/npm-release.yml); locally this
# only stages ./dist and runs `npm pack --dry-run`.
#
#   just npm-dist-local                                       # reassemble root from existing dist/
#   just npm-dist-local 0.1.0 path/to/kou x86_64-pc-windows-msvc
npm-dist-local version='' binary='' target='':
    KOU_SKIP_FONT_FETCH=1 just npm-dist kou {{version}} {{binary}} {{target}}
