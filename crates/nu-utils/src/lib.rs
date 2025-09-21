#![doc = include_str!("../README.md")]
mod casing;
mod deansi;
pub mod emoji;
pub mod filesystem;
pub mod flatten_json;
pub mod float;
pub mod locale;
mod multilife;
mod nu_cow;
mod quoting;
mod shared_cow;
mod split_read;
pub mod strings;
pub mod utils;

pub use locale::get_system_locale;
pub use utils::{
    ConfigFileKind, enable_vt_processing, get_ls_colors, stderr_write_all_and_flush,
    stdout_write_all_and_flush, terminal_size,
};

pub use casing::IgnoreCaseExt;
pub use deansi::{
    strip_ansi_likely, strip_ansi_string_likely, strip_ansi_string_unlikely, strip_ansi_unlikely,
};
pub use emoji::contains_emoji;
pub use flatten_json::JsonFlattener;
pub use float::ObviousFloat;
pub use multilife::MultiLife;
pub use nu_cow::NuCow;
pub use quoting::{escape_quote_string, needs_quoting};
pub use shared_cow::SharedCow;
pub use split_read::SplitRead;

#[cfg(unix)]
pub use filesystem::users;
