//! Private plugin **wire** mapping layer (the in-crate shim).
//!
//! Public protocol types (`PluginInput`, `PluginOutput`, …) stay the API that engine and
//! plugins use in Rust. Serde for those types goes through private wire DTOs so that:
//!
//! - Adding a top-level protocol variant requires updating an exhaustive match (compile fail).
//! - [`Value`](nu_protocol::Value) payloads go through [`value::WireValue`], so new `Value`
//!   variants also fail compile until mapped.
//! - Nested engine types (`ShellError`, `Config`, signatures, AST/IR, …) still use their
//!   existing derives; **protocol_snapshots** guard accidental byte drift for those.
//!
//! Wire types are intentionally private. Maintainer policy: if wire bytes change, bump
//! [`PLUGIN_PROTOCOL_VERSION`](crate::PLUGIN_PROTOCOL_VERSION) (0.x **minor** for breaks).

mod engine_call;
mod evaluated;
mod messages;
mod pipeline;
mod plugin_call;
mod protocol_info;
mod stream;
mod value;

// Submodules register Serialize/Deserialize for public protocol types.
