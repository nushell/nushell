# nu-plugin-protocol

This crate provides serde-compatible types for implementing the [Nushell plugin protocol](https://www.nushell.sh/contributor-book/plugin_protocol_reference.html). It is primarily used by the `nu-plugin` family of crates, but can also be used separately as well.

The specifics of I/O and serialization are not included in this crate. Use `serde_json` and/or `rmp-serde` (with the `named` serialization) to turn the types in this crate into data in the wire format.
