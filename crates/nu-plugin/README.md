# nu-plugin

## Updating Cap'n Proto schema

When modifying a protocol's struct that is used in plugins (such as Signature), you need to update the capnproto schema file and recompile it into the Rust source code.
The steps are as follows:
1. Change `src/serializers/capnp/schema/plugin.capnp` as necessary
2. Install `capnp` tool (it's a C++ binary, commonly available in package managers)
3. Install Rust support for `capnp` called `capnpc-rust`:
    1. `git clone https://github.com/capnproto/capnproto-rust` somewhere
    2. `cd capnproto-rust/capnpc`
    3. `cargo install --path=.`
4. Then, call `capnp compile -orust plugin.capnp` (change path as necessary)
5. The result should be `plugin_capnp.rs` file: Use it to replace the old `src/plugin_capnp.rs`.
6. It needs to be autoformatted (`cargo fmt --all`)
7. Modify the serialize/deserialize functions. Check the following PRs for details:
    * https://github.com/nushell/nushell/pull/4980
    * https://github.com/nushell/nushell/pull/4920

## Benchmark
Here is a simple benchmark for different protocol for encoding/decoding nushell table, with different rows and columns.  You can simply run `cargo bench` to run benchmark.

The relative html report is in `target/criterion/report/index.html`.
