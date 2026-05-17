#!/usr/bin/env bash

set -euo pipefail

echo ''
echo "----------------------------------------------"
echo "Uninstall nu and all plugins from cargo/bin..."
echo "----------------------------------------------"

NU_PLUGINS=(
    'nu_plugin_inc'
    'nu_plugin_gstat'
    'nu_plugin_query'
    'nu_plugin_example'
    'nu_plugin_formats'
    'nu_plugin_custom_values'
    'nu_plugin_polars'
)

cargo uninstall nu
for plugin in "${NU_PLUGINS[@]}"
do
    cargo uninstall "$plugin"
done
