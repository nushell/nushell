#!/bin/sh

# Usage: Just run `sh install-all.sh` in nushell root directory

echo "-----------------------------------------------------------------"
echo "Installing nushell (nu) with all the plugins"
echo "-----------------------------------------------------------------"
echo ""

echo "Install nushell from local..."
echo "----------------------------------------------"
cargo install --path .

NU_PLUGINS=(
    'nu_plugin_inc'
    'nu_plugin_gstat'
    'nu_plugin_query'
    'nu_plugin_example'
    'nu_plugin_custom_values'
)

for plugin in "${NU_PLUGINS[@]}"
do
    echo ''
    echo "----------------------------------------------"
    echo "Install plugin $plugin from local..."
    echo "----------------------------------------------"
    cd crates/$plugin && cargo install --path . && cd ../../
done
