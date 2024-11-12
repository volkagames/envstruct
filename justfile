set export

test *args='':
    cargo nextest run --run-ignored default $args

lint:
    cargo deny check advisories bans sources
    cargo fmt --all --check
    cargo check
    cargo clippy
