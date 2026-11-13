//! Re-exports signature-aware named flag helpers from `nu-protocol`.
//!
//! Kept as a thin module so existing `crate::named_flags::…` call sites stay stable.

pub use nu_protocol::engine::named_flags::*;
