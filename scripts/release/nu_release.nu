use std log

def publish [
    crate: path # the path to the crate to publish.
    --no-verify: bool # don’t verify the contents by building them. Can be useful for crates with a `build.rs`.
] {
    cd $crate

    if $no_verify {
        cargo publish --no-verify
    } else {
        cargo publish
    }
}

let subcrates_wave_1 = [
    nu-glob,
    nu-json,
    nu-path,
    nu-pretty-hex,
    nu-system,
    nu-utils,
    nu-term-grid,
    nu-test-support,
    nu-protocol,
    nu-engine,
    nu-plugin,
    nu-color-config,
    nu-parser,
    nu-table,
    nu-explore,
]

let subcrates_wave_2 = [
    nu-cmd-base,
    nu-cmd-lang,
    nu-cmd-dataframe,
    nu-cmd-extra,
    nu-command,
]

let subcrates_wave_3 = [
    nu-cli,
    nu-std,

    nu_plugin_query,
    nu_plugin_inc,
    nu_plugin_gstat,
    nu_plugin_formats,
]

log warning "publishing the first wave of crates"
for subcrate in $subcrates_wave_1 {
    publish ("crates" | path join $subcrate)
}

log warning "publishing the second wave of crates"
for subcrate in $subcrates_wave_2 {
    publish ("crates" | path join $subcrate) --no-verify
}

log warning "publishing the third wave of crates"
for subcrate in $subcrates_wave_3 {
    publish ("crates" | path join $subcrate)
}

cargo publish
