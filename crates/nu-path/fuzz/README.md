# Fuzzer for `nu-path`

- For detailed info, please look at [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)

# Quick start guide
- Install cargo-fuzz by `cargo install cargo-fuzz --locked`
- Make output directory `mkdir out`
- Run the fuzzer with `cargo fuzz run parse out`
