# Build and connect the installed binary to the local Herdr server.
install:
    #!/bin/sh
    set -eu
    command -v cargo >/dev/null 2>&1 || { echo 'Rust toolchain missing; install Rust 1.89+ from https://rustup.rs and rerun just install' >&2; exit 1; }
    cargo build --release --locked
    "${CARGO_TARGET_DIR:-target}/release/herdr-quota" install

# Compile a cleanup executable even if the installed program is already absent.
uninstall:
    #!/bin/sh
    set -eu
    command -v cargo >/dev/null 2>&1 || { echo 'Rust toolchain missing; install Rust 1.89+ from https://rustup.rs and rerun just uninstall' >&2; exit 1; }
    cargo build --release --locked
    "${CARGO_TARGET_DIR:-target}/release/herdr-quota" uninstall

# Repository checks, including isolated installation regression tests.
check:
    cargo fmt --all -- --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
