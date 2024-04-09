let toolchain_spec = open rust-toolchain.toml | get toolchain.channel
let msrv_spec = open Cargo.toml | get package.rust-version

# This check is conservative in the sense that we use `rust-toolchain.toml`'s
# override to ensure that this is the upper-bound for the minimum supported
# rust version
if $toolchain_spec != $msrv_spec {
    print -e "Mismatching rust compiler versions specified in `Cargo.toml` and `rust-toolchain.toml`"
    print -e $"Cargo.toml:          ($msrv_spec)"
    print -e $"rust-toolchain.toml: ($toolchain_spec)"
    exit 1
}
