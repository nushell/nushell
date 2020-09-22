use crate::config::{Conf, NuConfig, Status};
use nu_protocol::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FakeConfig {
    pub config: NuConfig,
    source_file: Option<PathBuf>,
}

impl Conf for FakeConfig {
    fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        self.is_modified()
    }

    fn var(&self, key: &str) -> Option<Value> {
        self.config.var(key)
    }

    fn env(&self) -> Option<Value> {
        self.config.env()
    }

    fn path(&self) -> Option<Value> {
        self.config.path()
    }

    fn reload(&mut self) {
        self.reload()
    }

    fn clone_box(&self) -> Box<dyn Conf> {
        self.config.clone_box()
    }
}

impl FakeConfig {
    pub fn new(config_file: &Path) -> FakeConfig {
        let config_file = Some(PathBuf::from(config_file));

        FakeConfig {
            config: NuConfig::with(config_file.clone()),
            source_file: config_file,
        }
    }

    pub fn is_modified(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let modified_at = &self.config.modified_at;

        Ok(
            match (NuConfig::get_last_modified(&self.source_file), modified_at) {
                (Status::LastModified(left), Status::LastModified(right)) => {
                    let left = left.duration_since(std::time::UNIX_EPOCH)?;
                    let right = (*right).duration_since(std::time::UNIX_EPOCH)?;

                    left != right
                }
                (_, _) => false,
            },
        )
    }

    pub fn reload(&mut self) {
        self.config = NuConfig::with(self.source_file.clone());
    }
}
