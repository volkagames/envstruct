set export

default:
    @just --list

lint:
    cargo deny check advisories bans sources
    cargo fmt --all --check
    cargo check
    cargo clippy
    cargo sort -c -w

fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-features --all-targets
    cargo fmt --all
    cargo sort -w

test *args='':
    cargo nextest run --run-ignored default $args
