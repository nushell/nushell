#!/usr/bin/env bash

set -euo pipefail

DIR=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))
REPO_ROOT=$(dirname $DIR)

echo "-----------------------------------------------------------------"
echo "Installing nushell (nu) and all the plugins"
echo "-----------------------------------------------------------------"
echo ""

echo "Install nushell from local..."
echo "----------------------------------------------"
cargo install --force --path "$REPO_ROOT" --locked

NU_PLUGINS=(
    'nu_plugin_inc'
    'nu_plugin_gstat'
    'nu_plugin_query'
    'nu_plugin_example'
    'nu_plugin_custom_values'
    'nu_plugin_formats'
    'nu_plugin_polars'
)

for plugin in "${NU_PLUGINS[@]}"
do
    echo ''
    echo "----------------------------------------------"
    echo "Install plugin $plugin from local..."
    echo "----------------------------------------------"
    cargo install --force --path "$REPO_ROOT/crates/$plugin"
done
