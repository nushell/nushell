print '-------------------------------------------------------------------'
print 'Building nushell (nu) with dataframes and all the plugins'
print '-------------------------------------------------------------------'

let repo_root = ($env.CURRENT_FILE | path dirname -n 2)

def build-nushell [] {
    print $'(char nl)Building nushell'
    print '----------------------------'

    cd $repo_root
    cargo build --features=dataframe
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
]

for plugin in $plugins {
    $plugin | build-plugin
}
