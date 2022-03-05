#!/bin/sh

echo "---------------------------------------------------------------"
echo "Building nushell (nu) with --features=extra and all the plugins"
echo "---------------------------------------------------------------"
echo ""

echo "Building nushell"
cargo build --features=extra
echo ""

cd crates/nu_plugin_example
echo "Building nu_plugin_example"
cargo build
echo ""

cd ../../crates/nu_plugin_gstat
echo "Building nu_plugin_gstat"
cargo build
echo ""

cd ../../crates/nu_plugin_inc
echo "Building nu_plugin_inc"
cargo build
echo ""

cd ../../crates/nu_plugin_query
echo "Building nu_plugin_query"
cargo build
echo ""

cd ../..
