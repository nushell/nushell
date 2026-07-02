//! Nushell configuration infrastructure.
//!
//! This crate owns **all** XDG-aware path resolution for Nushell's configuration
//! files.  It is the single source of truth for *where* config lives.  It does
//! NOT evaluate or parse config files — that is the job of `nu-cli`.
//!
//! ## Design
//!
//! [`resolve_paths`] is the one-shot entry point.  Call it once at startup with
//! the environment and CLI overrides, and you get back a [`NushellConfigDirs`]
//! struct with every path the shell needs.  No free functions, no ad-hoc env-var
//! reads spread across the codebase.
//!
//! ### Resolution priority
//!
//! For every path, the priority is:
//!
//! 1. CLI override (e.g. `--config-home`, `--config`, `--env-config`, `--plugin-config`)
//! 2. XDG environment variable (e.g. `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`)
//! 3. Platform default via the `dirs` crate

mod config_file;
mod env_access;
mod errors;
mod overrides;
mod paths;
mod resolve;

// Convenience re-export so callers don't need to dig into sub-modules.
pub use config_file::ConfigFileKind;
pub use env_access::{EnvAccess, SystemEnv, TestEnv};
pub use errors::{ConfigError, ConfigWarning};
pub use overrides::CliOverrides;
pub use paths::NushellConfigDirs;
pub use resolve::{config_home, resolve_paths};
