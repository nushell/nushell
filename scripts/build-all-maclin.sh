#!/usr/bin/env bash

set -euo pipefail

DIR=$(readlink -f $(dirname "${BASH_SOURCE[0]}"))
REPO_ROOT=$(dirname $DIR)

echo "---------------------------------------------------------------"
echo "Building nushell (nu) and all the plugins"
echo "---------------------------------------------------------------"
echo ""

NU_PLUGINS=(
    'nu_plugin_example'
    'nu_plugin_gstat'
    'nu_plugin_inc'
    'nu_plugin_query'
    'nu_plugin_custom_values'
    'nu_plugin_polars'
)

echo "Building nushell"
(
    cd $REPO_ROOT
    cargo build --locked
)

for plugin in "${NU_PLUGINS[@]}"
do
    echo "Building $plugin..."
    echo "-----------------------------"
    (
        cd "$REPO_ROOT/crates/$plugin"
        cargo build
    )
done
