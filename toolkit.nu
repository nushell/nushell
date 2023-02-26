export def fmt [--check: bool] {
    if ($check) {
        cargo fmt --all -- --check
    } else {
        cargo fmt --all
    }
}

export def clippy [] {
    cargo clippy --workspace -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect
}

export def test [--fast: bool] {
    if ($fast) {
        cargo nextest --workspace
    } else {
        cargo test --workspace
    }
}
