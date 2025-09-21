const nushell_dir = path self ..

# these crates should compile for wasm
const wasm_compatible_crates = [
    "nu-cmd-base",
    "nu-cmd-extra",
    "nu-cmd-lang",
    "nu-color-config",
    "nu-command",
    "nu-derive-value",
    "nu-engine",
    "nu-glob",
    "nu-json",
    "nu-parser",
    "nu-path",
    "nu-pretty-hex",
    "nu-protocol",
    "nu-std",
    "nu-system",
    "nu-table",
    "nu-term-grid",
    "nu-utils",
    "nuon"
]

def "prep wasm" [] {
    ^rustup target add wasm32-unknown-unknown
}

# build crates for wasm
export def "build wasm" [] {
    prep wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Building ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo build
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
        )
    }
}

# make sure no api is used that doesn't work with wasm
export def "clippy wasm" [] {
    prep wasm

    $env.CLIPPY_CONF_DIR = $nushell_dir | path join clippy wasm

    for crate in $wasm_compatible_crates {
        print $'(char nl)Checking ($crate) for wasm'
        print '----------------------------'
        (
            ^cargo clippy
                -p $crate
                --target wasm32-unknown-unknown
                --no-default-features
                --
                -D warnings
                -D clippy::unwrap_used
                -D clippy::unchecked_duration_subtraction
        )
    }
}
