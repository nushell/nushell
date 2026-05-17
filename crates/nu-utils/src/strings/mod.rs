//! String utility types with specific semantics.
//!
//! This module provides additional string types optimized for certain use-cases, offering
//! alternatives to standard [`String`] or [`&str`](str) when specific performance characteristics
//! are desired.
//!
//! The pipeline-based, functional programming model, we use, leads to frequent string operations
//! that involve immutability and often copying.
//! These specialized string types can provide performance and memory benefits for these cases.
//!
//! ## Choosing a String Type: `SharedString` vs. `UniqueString`
//!
//! ### Use [`SharedString`] when:
//!
//! `SharedString` is an owned, immutable string type optimized for **frequent cloning and sharing**.
//! Cloning it is very inexpensive (a pointer copy and atomic reference count increment), avoiding
//! deep copies of the string data.
//! It also benefits from Small String Optimization (SSO) and static string re-use.
//!
//! **Ideal for:** Strings that need to be duplicated or passed by value frequently across
//! pipeline stages or within complex data structures, where many references to the same string
//! data are expected.
//!
//! ### Use [`UniqueString`] when:
//!
//! `UniqueString` is an owned, immutable string type optimized for
//! **strings that are primarily unique** or rarely cloned.
//! Cloning a `UniqueString` always involves copying the underlying string data.
//! Its advantage lies in avoiding the overhead of atomic reference counting.
//! It also benefits from Small String Optimization (SSO) and static string re-use.
//!
//! **Ideal for:** Strings that are created and consumed locally, or represent unique identifiers
//! that are not expected to be duplicated across many ownership boundaries.
//! When the cost of copying upon infrequent clones is acceptable.

mod shared;
mod unique;

pub use shared::SharedString;
pub use unique::UniqueString;
