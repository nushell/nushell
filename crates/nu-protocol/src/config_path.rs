use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Specifies a path to a configuration file and its type
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigPath {
    /// Path to the global configuration file
    Global(PathBuf),
    /// Path to a local configuration file
    Local(PathBuf),
}

impl ConfigPath {
    pub fn get_path(&self) -> &PathBuf {
        match self {
            ConfigPath::Global(p) | ConfigPath::Local(p) => p,
        }
    }
}
