//! Explicit serde implementation for plugin protocol types.
//!
//! This module intentionally maps public protocol types to private serde helper
//! representations so protocol wire changes are explicit in review.
//!
//! Implementation lives in themed submodules so this directory stays reviewable.

mod engine_call;
mod evaluated;
mod messages;
mod pipeline;
mod plugin_call;
mod protocol_info;
mod stream;
