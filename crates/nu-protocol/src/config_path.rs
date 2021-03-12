use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfigPath {
    Global(PathBuf),
    Local(PathBuf),
}

impl ConfigPath {
    pub fn get_path(&self) -> &PathBuf {
        match self {
            ConfigPath::Global(p) => p,
            ConfigPath::Local(p) => p,
        }
    }
}
