fmt-check:
	cargo fmt --all -- --check

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect

test:
	cargo test --workspace
