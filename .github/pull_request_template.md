Thanks for your pull request! We appreciate the support. Here are a few steps that will be checked automatically on every pull request. Making sure each of these works locally will help your PR go through with less trouble.

Make sure you've run and fixed any issues with these commands:

- [ ] `cargo fmt` to give the code standard formatting
- [ ] `cargo clippy --all --all-features -- -D warnings -D clippy::unwrap_used -A clippy::needless_collect` to check that you're using the standard code style
- [ ] `cargo build; cargo test --all --all-features` to check that all the tests pass
