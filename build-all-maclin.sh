#!/bin/sh

echo "---------------------------------------------------------------"
echo "Building nushell (nu) with --features=extra and all the plugins"
echo "---------------------------------------------------------------"
echo ""

NU_PLUGINS=(
    'nu_plugin_example'
    'nu_plugin_gstat'
    'nu_plugin_inc'
    'nu_plugin_query'
    'nu_plugin_custom_values'
)

echo "Building nushell"
cargo build --features=extra
for plugin in "${NU_PLUGINS[@]}"
do
    echo '' && cd crates/$plugin
    echo "Building $plugin..."
    echo "-----------------------------"
    cargo build && cd ../..
done
