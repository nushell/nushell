pub mod group;
pub mod split;
pub mod tests;

pub use crate::utils::data::group::group;
pub use crate::utils::data::split::split;

pub use crate::utils::data::tests::{report, Operation};
