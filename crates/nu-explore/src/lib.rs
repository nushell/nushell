#![doc = include_str!("../README.md")]
mod default_context;
mod explore;
mod explore_config;
mod explore_regex;

pub use default_context::add_explore_context;
pub use explore::{Explore, ExploreConfig};
pub use explore_regex::ExploreRegex;
