#![doc = include_str!("../README.md")]
mod assert_path_eq;
mod components;
pub mod dots;
pub mod expansions;
pub mod form;
mod helpers;
mod path;
mod tilde;
mod trailing_slash;

pub use components::components;
pub use expansions::{
    canonicalize_with, expand_path, expand_path_with, expand_to_real_path, locate_in_dirs,
};
pub use helpers::{cache_dir, data_dir, home_dir, nu_config_dir};
pub use path::*;
pub use tilde::expand_tilde;
pub use trailing_slash::{has_trailing_slash, strip_trailing_slash};
