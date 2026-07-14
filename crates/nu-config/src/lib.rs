//! Nushell configuration infrastructure.
//!
//! This crate owns **all** XDG-aware path resolution for Nushell configuration
//! files and directories. It is the single source of truth for *where* config
//! lives. It does **not** evaluate or parse config files — that is the job of
//! `nu-cli` / the `nu` binary.
//!
//! # Architecture
//!
//! ```text
//! CLI flags + process env + platform dirs
//!              │
//!              ▼
//!      resolve_paths()          ← one-shot at startup
//!              │
//!              ▼
//!     NushellConfigDirs         ← stored on EngineState.config_dirs
//!              │
//!     ┌────────┼────────┬──────────────┐
//!     ▼        ▼        ▼              ▼
//!  $nu.*   loaders   history       plugins
//! ```
//!
//! After startup, **do not** re-read `XDG_*` env vars or call free path helpers.
//! Always read from [`NushellConfigDirs`].
//!
//! # Resolution priority
//!
//! For every path:
//!
//! 1. CLI override ([`CliOverrides`] — e.g. `--config-home`, `--config`)
//! 2. XDG environment variable (e.g. `$XDG_CONFIG_HOME`)
//! 3. Platform default via the [`EnvAccess`] seam
//!
//! # Testing
//!
//! Use [`TestEnv`] to inject env vars and platform directories without touching
//! the host process. Prefer unit tests in this crate over spawning `nu` for pure
//! path-resolution logic.

mod config_file;
mod env_access;
mod errors;
mod overrides;
mod paths;
mod resolve;

// Convenience re-exports so callers don't need to dig into sub-modules.
pub use config_file::ConfigFileKind;
pub use env_access::{EnvAccess, SystemEnv, TestEnv};
pub use errors::{ConfigError, ConfigWarning};
pub use overrides::CliOverrides;
pub use paths::{ConfigPath, NushellConfigDirs};
pub use resolve::resolve_paths;

#[cfg(test)]
#[macro_use]
extern crate nu_test_support;

#[cfg(test)]
use nu_test_support::harness::main;
