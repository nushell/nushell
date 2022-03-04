#!/bin/sh

cargo install --path . --features=extra

NU_PLUGINS=(
    'nu_plugin_inc'
    'nu_plugin_gstat'
    'nu_plugin_query'
    'nu_plugin_example'
)
for plugin in "${NU_PLUGINS[@]}"
do
    echo ''
    echo "----------------------------------------------"
    echo "Install plugin $plugin from local..."
    echo "----------------------------------------------"
    cd crates/$plugin && cargo install --path . && cd ../../
done

# Uncomment the followding lines to UNINSTALL:
# echo '' && cargo uninstall nu
# for plugin in "${NU_PLUGINS[@]}"
# do
#     cargo uninstall $plugin
# done
