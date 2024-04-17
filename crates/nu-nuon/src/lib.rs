//! Support for the NUON format.
//!
//! The NUON format is a superset of JSON designed to fit the feel of Nushell.
//! Some of its extra features are
//! - trailing commas are allowed
//! - quotes are not required around keys
mod from;
mod to;

pub use from::from_nuon;
pub use to::to_nuon;
