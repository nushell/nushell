use std/log warning

print '-------------------------------------------------------------------'
print 'Building nushell (nu) and all the plugins'
print '-------------------------------------------------------------------'

warning "./scripts/build-all.nu will be deprecated, please use the `toolkit build` command instead"

let repo_root = ($env.CURRENT_FILE | path dirname --num-levels 2)

def build-nushell [] {
    print $'(char nl)Building nushell'
    print '----------------------------'

    cd $repo_root
    cargo build --locked
}

def build-plugin [] {
    let plugin = $in

    print $'(char nl)Building ($plugin)'
    print '----------------------------'

    cd $'($repo_root)/crates/($plugin)'
    cargo build
}

let plugins = [
    nu_plugin_inc,
    nu_plugin_gstat,
    nu_plugin_query,
    nu_plugin_example,
    nu_plugin_custom_values,
    nu_plugin_formats,
    nu_plugin_polars
]

for plugin in $plugins {
    $plugin | build-plugin
}
