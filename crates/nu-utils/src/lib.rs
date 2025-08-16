#![doc = include_str!("../README.md")]
mod casing;
mod deansi;
pub mod emoji;
pub mod filesystem;
pub mod flatten_json;
pub mod float;
pub mod locale;
mod quoting;
mod shared_cow;
pub mod strings;
pub mod utils;

pub use locale::get_system_locale;
pub use utils::{
    enable_vt_processing, get_default_config, get_default_env, get_doc_config, get_doc_env,
    get_ls_colors, get_scaffold_config, get_scaffold_env, stderr_write_all_and_flush,
    stdout_write_all_and_flush, terminal_size,
};

pub use casing::IgnoreCaseExt;
pub use deansi::{
    strip_ansi_likely, strip_ansi_string_likely, strip_ansi_string_unlikely, strip_ansi_unlikely,
};
pub use emoji::contains_emoji;
pub use flatten_json::JsonFlattener;
pub use float::ObviousFloat;
pub use quoting::{escape_quote_string, needs_quoting};
pub use shared_cow::SharedCow;

#[cfg(unix)]
pub use filesystem::users;
