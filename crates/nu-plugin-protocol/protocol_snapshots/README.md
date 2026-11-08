# Plugin protocol snapshots

Plugins and the engine talk over a serialized wire protocol. A plugin built against one Nushell release keeps working with later releases as long as both sides speak a compatible plugin protocol version. That version is `PLUGIN_PROTOCOL_VERSION` in [`src/protocol_info.rs`](../src/protocol_info.rs). It is separate from the Nushell version, so it only changes when the wire does.

This directory holds the snapshots that tell you when the wire changed:

| File | What it is |
| --- | --- |
| `plugin_input.json` | Every engine-to-plugin message in the frozen tier, as JSON |
| `plugin_output.json` | Every plugin-to-engine message in the frozen tier |
| `variants.json` | Every variant of the enums the messages only partly use (`Category`, `Operator`, `Casing`, `SuggestionKind`, `CommandType`, `CommandWideCompleter`) |
| `frozen/<version>/` | The same three files as written for that protocol version, never regenerated |

All of them are generated from the corpus in [`src/tests.rs`](../src/tests.rs). Don't edit them by hand.

## How compatibility is decided

Each side sends a `Hello` first. It carries:

- `version`: the plugin protocol version the side speaks.
- `nushell_version`: the version of the Nushell crates the side was built from. Rust plugins send it automatically. Plugins written in other languages leave it out.

Two sides are compatible when the higher protocol version matches a semver caret requirement on the lower one. On `0.x` that means the same minor version: `0.1.0` and `0.1.3` are compatible, `0.1.3` and `0.2.0` are not. Prerelease labels such as `0.1.0-nightly.1` are ignored.

If the versions aren't compatible, the side that notices fails the handshake and the plugin doesn't load:

```text
Plugin `regex` speaks plugin protocol 0.111.0, which is not compatible with this
engine's protocol 0.1.0. Rebuild the plugin against this version of Nushell
```

`nushell_version` never affects the handshake. It only decides whether the engine-internal tier below can be used.

## The three tiers

Every type that crosses the wire belongs to one of three tiers. The tier decides how a change to the type is handled.

### Frozen

**What's in it.** The messages defined in this crate (`PluginInput`, `PluginOutput`, `PluginCall`, `EngineCall`, their responses, stream messages, `ProtocolInfo`), `Value`, and the `nu-protocol` types inside them: spans, errors (`ShellError` travels as `LabeledError`), pipeline metadata, signatures, `Type`, `SyntaxShape`, completion suggestions, and plugin metadata.

**What it's for.** This is the contract. A plugin built at protocol `0.1.0` must be able to read everything a `0.1.x` engine sends it, and the other way around.

**How it's checked.** Three ways, from fastest to most thorough:

1. Compile-time witnesses. [`src/wire_witness.rs`](../src/wire_witness.rs) and `nu-protocol`'s [`src/plugin/wire_witness.rs`](../../nu-protocol/src/plugin/wire_witness.rs) name every variant and field of each frozen type with derived serde, in exhaustive matches and destructures. Adding, removing, or renaming a variant or field stops the build in that file. Fields are matched as `_`, so a changed field type or serde attribute still compiles; the snapshots catch those.
2. Snapshots. `messages_match_snapshots` serializes the corpus and compares it, line by line, with `plugin_input.json`, `plugin_output.json`, and `variants.json`. Any change to the encoding fails, including the order of keys and of record columns. This is also the only check for the types with hand-written serde (`Record`, `Range`, `CellPath`, `PathMember`, the `Id` types, `ShellError`) or private fields (`Filesize`, `OneOf`, `CollectionColumns`), and only for the values in the corpus.
3. Frozen fixtures. `frozen_fixtures_decode` reads `frozen/<version>/` for every version compatible with the current one. Each entry must still decode, and encoding it again must keep every key and value the fixture has. So a peer at that version can still read what we send and still be read by us.

**How it stays compatible.** Every change to a frozen type either keeps the old encoding readable in both directions or bumps the protocol version (see the table below). The frozen fixtures hold the old encodings, so a change that breaks an older compatible peer fails the tests.

**When it refuses.** A peer on another minor line (on `0.x`) fails the handshake. Nothing in this tier is ever sent to an incompatible peer.

### Tolerant

**What's in it.** The engine `Config`, sent in reply to `GetConfig`.

**What it's for.** Config options change in almost every Nushell release. Freezing `Config` would force a protocol bump each time, so instead it travels in a form that tolerates differences.

**How it's checked.** The engine sends the config as a `Value` record, built with `Config::into_value`. The plugin side starts from `Config::default()` and applies the record with `Config::update_from_value`, the same code that handles `$env.config`. The `config_record_*` tests in [`src/tests.rs`](../src/tests.rs) check the round trip, unknown and missing options, and that a payload that isn't a record is an error. The config isn't part of the snapshots.

**How it stays compatible.** Options the plugin doesn't know are skipped, and options the engine didn't send keep the plugin's defaults. Two things differ from a `$env.config` edit. The record's keybinding list replaces the default bindings instead of merging into them. And the plugin keeps the `history.file_format` the record names, because without nu-protocol's `sqlite` feature, which plugins built outside the workspace don't enable, the parser would read a sqlite history as plaintext.

**When it refuses.** Never. A config change never needs a protocol bump.

### Engine-internal

**What's in it.** IR blocks (the `GetBlockIR` engine call and its `IrBlock` response), and the parsed `ast::Call` sent with `PluginCall::GetCompletion` for dynamic completion.

**What it's for.** These types change with the parser and the compiler, sometimes in every release. They can't be frozen without freezing the compiler, so they are outside the protocol version.

**How it's checked.** They aren't in the snapshots or the witnesses. The engine only exchanges them when both `Hello`s report the same `nushell_version`, compared as exact strings. `same_nushell_version_requires_both_versions_to_match` tests the comparison. The engine's gates are tested by `interface_handle_engine_call_get_block_ir_requires_same_nushell_version` and `interface_get_dynamic_completion_skips_plugin_from_other_nushell_version` in `nu-plugin-engine`.

**How it stays compatible.** It doesn't try to. A plugin built from the same Nushell version as the engine gets IR and dynamic completion. Every other plugin keeps working, without them.

**When it refuses.** Whenever the Nushell versions differ or one side doesn't report one, which includes every plugin not written in Rust. `get_block_ir` then returns an error that points at the plugin call, and dynamic completion falls back to the default completions. The rest of the plugin is unaffected.

## What forces a protocol bump

Only changes to frozen types. Use the first row that matches:

| Change to a frozen type | Bump |
| --- | --- |
| Remove or rename a variant or field, or change what it means | minor (`0.1` → `0.2`) |
| Add a required field, or change a field's type | minor |
| Add a variant to an enum inside a message (`Value`, `Category`, `SyntaxShape`, ...) | minor |
| Add an element to a tuple or newtype variant | minor |
| Add a named field that is `Option` or `#[serde(default)]` | patch |
| Add a message that the sending interface only sends after checking the peer's `Hello` | patch |
| Add or remove a `#[serde(skip)]` field | none |
| Documentation only | none |

Why so few patch cases: an older peer fails to decode an unknown variant or an extra tuple element, and that ends the connection. Only the interfaces in `nu-plugin` and `nu-plugin-engine` see the peer's `Hello`. Plugin code that builds a `Signature` or a `Value` doesn't, so it can't hold a new variant back from an older engine. That leaves two additions an older peer survives: named fields it skips, and messages it is never sent.

Also watch for changes that reach the wire indirectly. `Range` and `CellPath` travel as display strings that share formatting and quoting code with `to nuon`, and `Record` has hand-written serde. A change there is a frozen-tier change even though no protocol type was touched. The snapshots catch it for the values in the corpus.

## How to bump

1. Make the change. The build stops at a witness, or `messages_match_snapshots` fails.
2. Pick the bump from the table. Bump at most once per Nushell release. To see whether the current version has shipped, look at it in the last release, with `git show <release tag>:crates/nu-plugin-protocol/src/protocol_info.rs`. If it hasn't shipped, delete its `frozen/<version>/` directory, since no peer ever spoke it. Then keep that version if its bump is at least as big as the one you need, or bump again from the last shipped version.
3. Change `PLUGIN_PROTOCOL_VERSION` in [`src/protocol_info.rs`](../src/protocol_info.rs) and the copies in the plugins that can't import it:
   - `crates/nu_plugin_python/nu_plugin_python_example.py`
   - `crates/nu_plugin_javascript/nu_plugin_node_example.js`
   - `crates/nu_plugin_nu_example/nu_plugin_nu_example.nu`
   - `crates/nu_plugin_stress_internals/src/main.rs`

   `tests/plugins/protocol_version.rs` fails until all four match.
4. Update the witness that stopped the build, and add the new variants, fields, or messages to the corpus in [`src/tests.rs`](../src/tests.rs) so the snapshots cover them.
5. Regenerate:

   ```sh
   cargo test -p nu-plugin-protocol -- --ignored regenerate_protocol_snapshots
   ```

   This rewrites the three snapshot files and creates `frozen/<new version>/`. It never overwrites a frozen file that already exists.
6. Check:

   ```sh
   cargo test -p nu-plugin-protocol
   cargo test --test tests -- plugins
   ```

   After a minor bump, the older `frozen/` directories are no longer compatible and `frozen_fixtures_decode` skips them. Keep them as a record of what older plugins send.
7. Commit the snapshots and the new `frozen/` directory with the change, and mention the new protocol version in the release notes so plugin authors know whether to rebuild.

A change that needs no bump (a `#[serde(skip)]` field, documentation) leaves the snapshots unchanged. If a test that only adds coverage changes the corpus, regenerate as in step 5; the frozen files stay as they are.

## When a check fails

- **The build stops in a `wire_witness.rs`.** A frozen variant or field was added, removed, or renamed. Follow "How to bump".
- **`messages_match_snapshots` fails.** The encoding changed. The message shows the first line that differs. If the change is intended, follow "How to bump".
- **`frozen_fixtures_decode` says an entry "no longer decodes".** A message that compatible peers still send can't be read. That needs a minor bump, or the change should keep the old form readable.
- **`frozen_fixtures_decode` says an entry "encodes differently now".** We still read the old message but would send it in a form older peers read differently: a field or variant was renamed or removed. That needs a minor bump. If the only difference is a field that is now left out when empty (`skip_serializing_if`), check that peers at that version accept it missing.

## Examples

Each example below was applied to this tree, and `cargo test -p nu-plugin-protocol` was run against it. The output is copied from those runs, with paths shortened, long lines wrapped, and long messages cut where marked `...`. Line numbers will drift as the files change. The last example wasn't run; it follows from how serde encodes the two types.

Two patterns run through them. The build stops at a witness when a variant or field is added, removed, or renamed. The tests fail when the encoding of something in the corpus changes. Only a change that breaks the old fixtures keeps failing until the version moves to a new minor line; for additions, the build stop and the snapshot diff in review are the signal to bump.

### Add a variant to an enum inside a message: minor

```rust
// crates/nu-protocol/src/signature.rs
pub enum Category {
    // ...
    Viewers,
    Ai, // new
}
```

The witness stops the build:

```text
error[E0004]: non-exhaustive patterns: `&signature::Category::Ai` not covered
   --> crates/nu-protocol/src/plugin/wire_witness.rs:282:43
    |
282 | const _: fn(&Category) = |category| match category {
    |                                           ^^^^^^^^ pattern `&signature::Category::Ai` not covered
```

A plugin built with this change can put a command in `Category::Ai`. A `0.1.0` engine can't decode that signature, so `plugin add` fails. Bump to `0.2.0`, add `Category::Ai` to the witness and to `every_variant()` in `src/tests.rs`, and regenerate.

Once the witness lists `Ai`, the tests pass whether or not the version was bumped: the frozen fixtures were written before `Ai` existed, so nothing in them changes. The build stop is the reminder.

### Add an optional field: patch

```rust
// crates/nu-protocol/src/pipeline/metadata.rs
pub struct PipelineMetadata {
    pub data_source: DataSource,
    #[serde(default)]
    pub path_columns: Vec<String>,
    pub content_type: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>, // new
    #[serde(default)]
    pub custom: Record,
}
```

The witness stops the build (so does `PipelineMetadata::for_collect`, which takes the struct apart too):

```text
error[E0027]: pattern does not mention field `encoding`
   --> crates/nu-protocol/src/plugin/wire_witness.rs:136:9
    |
136 |       let PipelineMetadata {
    |  _________^
137 | |         data_source: _,
138 | |         path_columns: _,
139 | |         content_type: _,
140 | |         custom: _,
141 | |     } = metadata;
    | |_____^ missing field `encoding`
```

After adding `encoding: _` to the witness and `encoding: Some("utf-8".into())` to `full_metadata()` in the corpus, only the snapshot test fails:

```text
protocol_snapshots/plugin_input.json changed on the wire at line 644:
  snapshot:                 "custom": {
  current:                  "encoding": "utf-8",
```

`frozen_fixtures_decode` passes. A `0.1.0` message without `encoding` decodes with `None`, and a `0.1.0` peer skips the `encoding` key it doesn't know. That's what makes it a patch: change `PLUGIN_PROTOCOL_VERSION` and its four copies to `0.1.1`, then regenerate, which also writes `frozen/0.1.1/`. Nothing fails if the patch bump is forgotten; the snapshot diff is the signal.

### Add a required field: minor

The same field without `#[serde(default)]` and `Option`:

```rust
    pub encoding: String, // new, required
```

The witness stops the build as above. After the witness and the corpus are updated, the frozen fixtures fail too:

```text
protocol_snapshots/frozen/0.1.0/plugin_input.json[4] no longer decodes: missing field `encoding`.
Peers that speak this protocol version still send it, so the change needs a PLUGIN_PROTOCOL_VERSION
minor bump.
```

This failure stays until the version moves to `0.2.0`. Then `frozen/0.1.0/` is no longer compatible and is skipped, and regenerating writes `frozen/0.2.0/`.

### Add an element to a tuple variant: minor

```rust
// crates/nu-protocol/src/completion.rs
pub enum SuggestionKind {
    Command(CommandType, Option<DeclId>, bool), // was Command(CommandType, Option<DeclId>)
    // ...
}
```

The witness stops the build:

```text
error[E0023]: this pattern has 2 fields, but the corresponding tuple variant has 3 fields
   --> crates/nu-protocol/src/plugin/wire_witness.rs:410:29
    |
410 |     SuggestionKind::Command(_, _)
    |                             ^  ^ expected 3 fields, found 2
```

After the witness and the corpus are updated, the old two-element array no longer decodes:

```text
protocol_snapshots/frozen/0.1.0/plugin_output.json[10] no longer decodes: invalid length 2, expected
tuple variant SuggestionKind::Command with 3 elements. Peers that speak this protocol version still
send it, so the change needs a PLUGIN_PROTOCOL_VERSION minor bump.
```

Tuple variants encode as arrays, so there's no optional element. If you need a patch-level addition, use a variant with named fields from the start.

### Rename a variant on the wire: minor

Changing only the serde name leaves the Rust code, and so the witness, untouched:

```rust
// crates/nu-protocol/src/signature.rs
pub enum Category {
    // ...
    #[serde(rename = "filter")]
    Filters,
    // ...
}
```

Both tests fail:

```text
protocol_snapshots/variants.json changed on the wire at line 21:
  snapshot:       "Filters",
  current:        "filter",
```

```text
protocol_snapshots/frozen/0.1.0/variants.json[0] no longer decodes: unknown variant `Filters`, expected
one of `Bits`, `Bytes`, ..., `filter`, ... Peers that speak this protocol version still send it, so the
change needs a PLUGIN_PROTOCOL_VERSION minor bump.
```

Adding `alias = "Filters"` makes the old name decode again, but it doesn't make the change compatible: a `0.1.0` peer still can't read `"filter"`. The frozen check catches that as well:

```text
protocol_snapshots/frozen/0.1.0/variants.json[0] encodes differently now:
...
Peers that speak this protocol version read it the old way, so the change needs a
PLUGIN_PROTOCOL_VERSION minor bump. ...
```

### Change a type with hand-written serde: minor, or a bug

`Range` travels as its display string, so a change to how ranges print is a wire change, and no witness can see it:

```rust
// crates/nu-protocol/src/value/range.rs, impl Display for IntRange
if self.step != 1 {
    write!(f, "{}..", self.step)?; // was self.start + self.step
}
```

```text
protocol_snapshots/plugin_input.json changed on the wire at line 113:
  snapshot:                   "val": "1..3..10",
  current:                    "val": "1..2..10",
```

```text
protocol_snapshots/frozen/0.1.0/plugin_input.json[3] encodes differently now:
{
  "Call": [
    2,
    ...
Peers that speak this protocol version read it the old way, so the change needs a
PLUGIN_PROTOCOL_VERSION minor bump. ...
```

The old message still decodes, because the parser didn't change, but the range `1..3..10` would now be sent as `1..2..10`, which a `0.1.0` plugin reads as a different range. Here the right fix is to undo the change; a failing frozen check can mean the change is wrong, not that it needs a bump.

### Add a message: patch, if it's gated

```rust
// crates/nu-plugin-protocol/src/lib.rs
pub enum EngineCall<D> {
    // ...
    GetBlockIR(BlockId),
    GetPluginVersion, // new
    // ...
}
```

This crate's witness stops the build. So do `EngineCall::name` and `map_data` in the same file and the handlers in `nu-plugin` and `nu-plugin-engine`, which have to learn the call anyway; the witness is the one that points to the bump table:

```text
error[E0004]: non-exhaustive patterns: `&EngineCall::GetPluginVersion` not covered
   --> crates/nu-plugin-protocol/src/wire_witness.rs:131:45
    |
131 | const _: fn(&EngineCall<()>) = |call| match call {
    |                                             ^^^^ pattern `&EngineCall::GetPluginVersion` not covered
```

A `0.1.0` engine can't decode `GetPluginVersion`, so it's a patch only if the plugin side never sends it to such an engine. The existing tool for that is a `Feature`: it's the one enum with a catch-all (`#[serde(other)] Unknown`), so older peers read a feature name they don't know instead of failing. A sketch, not compiled:

```rust
// crates/nu-plugin-protocol/src/protocol_info.rs
pub enum Feature {
    LocalSocket,
    GetPluginVersion, // engines that handle the call list it in their Hello
    #[serde(other, skip_serializing)]
    Unknown,
}
// ... and add it to `default_features()` and `Feature::is_compatible_with`.

// crates/nu-plugin/src/plugin/interface/mod.rs, impl EngineInterface
pub fn get_plugin_version(&self) -> Result<String, ShellError> {
    let engine = self.state.protocol_info.get()?;
    if !engine.is_some_and(|info| info.supports_feature(&Feature::GetPluginVersion)) {
        return Err(ShellError::Generic(GenericError::new_internal(
            "get_plugin_version is not supported",
            "this version of Nushell doesn't handle it",
        )));
    }
    match self.engine_call(EngineCall::GetPluginVersion)? {
        // ...
    }
}
```

For a new `PluginCall` going the other way, the engine side can check the plugin's `Hello` the same way through `PluginInterface::protocol_info`. Add the message to `plugin_output_messages()` (or `plugin_input_messages()`) in the corpus and regenerate. As with the optional field, the tests don't force the patch bump. Without the gate, the change is a minor bump.

### Changes that need no bump

- A `#[serde(skip)]` field, such as `#[serde(skip)] pub source_line: Option<u32>` on `PluginExample`. The witness stops the build like any new field. After the witness is updated, all tests pass and the snapshots don't change, because the field never reaches the wire.
- A new config option. `Config` is in the tolerant tier, so it has no witness and isn't in the snapshots. Nothing fails, and nothing should: plugins that don't know the option skip it.
- A new IR instruction or a change to `ast::Call`. These are engine-internal, so nothing checks them, and only plugins built from the same Nushell version ever receive them.

### A change the checks miss

```rust
// crates/nu-protocol/src/signature.rs
pub struct Flag {
    pub long: String,
    pub short: Option<String>, // was Option<char>
    // ...
}
```

The witness matches fields as `_`, so it still compiles. The corpus has the short flag `'n'`, and a `char` and a one-letter `String` both encode as the JSON string `"n"`, so the snapshots and the frozen fixtures still pass. But a plugin that declares a two-letter short flag would send `"nn"`, which a `0.1.0` engine can't decode as a `char`. That's a minor bump that only review catches. The same goes for any type change whose effect doesn't show in the values the corpus holds, which is why new variants and edge-case values belong in the corpus.
