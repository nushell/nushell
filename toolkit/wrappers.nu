
# Run nushell from source with a right indicator.
@category "toolkit"
@search-terms run cargo run source development dev
@example "Run Nushell from source" { toolkit dev }
@example "Run with experimental options enabled" { toolkit dev --experimental-options "dataframe" }
export def dev [
    --experimental-options: oneof<list<string>, string> # enable or disable experimental options
] {
    let experimental_options_arg = $experimental_options
        | default []
        | [$in]
        | flatten
        | str join ","
        | $"[($in)]"

    ^cargo run -- ...[
        --experimental-options $experimental_options_arg
        -e "$env.PROMPT_COMMAND_RIGHT = $'(ansi magenta_reverse)trying Nushell inside Cargo(ansi reset)'"
    ]
}

def build-nushell [features: string] {
    print $'(char nl)Building nushell'
    print '----------------------------'

    ^cargo build --features $features --locked
}

def build-plugin [] {
    let plugin = $in

    print $'(char nl)Building ($plugin)'
    print '----------------------------'

    cd $"crates/($plugin)"
    ^cargo build
}

# Build Nushell and plugins with some features.
@category "toolkit"
@search-terms build compile cargo build features plugins
@example "Build Nushell with default features" { toolkit build }
@example "Build Nushell and all plugins" { toolkit build --all }
@example "Build with specific features" { toolkit build extra stable --all }
export def build [
    ...features: string@"nu-complete list features"  # a space-separated list of feature to install with Nushell
    --all # build all plugins with Nushell
] {
    build-nushell ($features | default [] | str join ",")

    if not $all {
        return
    }

    let plugins = [
        nu_plugin_inc,
        nu_plugin_gstat,
        nu_plugin_query,
        nu_plugin_polars,
        nu_plugin_example,
        nu_plugin_custom_values,
        nu_plugin_formats,
    ]

    for plugin in $plugins {
        $plugin | build-plugin
    }
}

def "nu-complete list features" [] {
    open Cargo.toml | get features | transpose feature dependencies | get feature
}

def install-plugin [] {
    let plugin = $in

    print $'(char nl)Installing ($plugin)'
    print '----------------------------'

    ^cargo install --path $"crates/($plugin)"
}

# Install Nushell and features you want.
@category "toolkit"
@search-terms install cargo install features plugins
@example "Install Nushell with default features" { toolkit install }
@example "Install Nushell and all plugins" { toolkit install --all }
@example "Install with specific features" { toolkit install extra stable --all }
export def install [
    ...features: string@"nu-complete list features"  # a space-separated list of feature to install with Nushell
    --all # install all plugins with Nushell
] {
    touch crates/nu-cmd-lang/build.rs # needed to make sure `version` has the correct `commit_hash`
    ^cargo install --path . --features ($features | default [] | str join ",") --locked --force
    if not $all {
        return
    }

    let plugins = [
        nu_plugin_inc,
        nu_plugin_gstat,
        nu_plugin_query,
        nu_plugin_polars,
        nu_plugin_example,
        nu_plugin_custom_values,
        nu_plugin_formats,
    ]

    for plugin in $plugins {
        $plugin | install-plugin
    }
}
