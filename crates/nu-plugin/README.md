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
1. It's recommended to use cargo-criterion to run benchmark for future analysing:
```
cargo install cargo-criterion
```

2. then run the following commands to generate benchmark result:
```
cargo criterion --message-format=json | save "result.json"
```

3. filter noisy data from cargo-criterion:
```
let aa = (open result.json | lines | where (not ($it | str contains "group-complete")))
("[" + ($aa | str collect ',') + "]") | save -r result.json
```

4. open `result.json` file and do anything you want to gather benchmark result, e.g:
```
open bb.json | select id mean | flatten mean
```
