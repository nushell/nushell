#![doc = include_str!("../README.md")]
mod casing;
mod deansi;
pub mod emoji;
pub mod filesystem;
pub mod locale;
mod quoting;
mod shared_cow;
pub mod utils;

pub use locale::get_system_locale;
pub use utils::{
    enable_vt_processing, get_default_config, get_default_env, get_ls_colors, get_sample_config,
    get_sample_env, get_scaffold_config, get_scaffold_env, stderr_write_all_and_flush,
    stdout_write_all_and_flush,
};

pub use casing::IgnoreCaseExt;
pub use deansi::{
    strip_ansi_likely, strip_ansi_string_likely, strip_ansi_string_unlikely, strip_ansi_unlikely,
};
pub use emoji::contains_emoji;
pub use quoting::{escape_quote_string, needs_quoting};
pub use shared_cow::SharedCow;

#[cfg(unix)]
pub use filesystem::users;
