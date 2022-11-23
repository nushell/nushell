echo '-------------------------------------------------------------------'
echo 'Building nushell (nu) with dataframes and all the plugins'
echo '-------------------------------------------------------------------'

echo $'(char nl)Building nushell'
echo '----------------------------'
cargo build --features=dataframe

let plugins = [
    nu_plugin_inc,
    nu_plugin_gstat,
    nu_plugin_query,
    nu_plugin_example,
    nu_plugin_custom_values,
]

for plugin in $plugins {
    $'(char nl)Building ($plugin)'
    '----------------------------'
    cd $'crates/($plugin)'
    cargo build
    ignore
}
