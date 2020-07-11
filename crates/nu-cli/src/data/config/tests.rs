use crate::data::config::{read, Conf, NuConfig};
use indexmap::IndexMap;
use nu_protocol::Value;
use nu_source::Tag;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct FakeConfig {
    pub config: NuConfig,
}

impl Conf for FakeConfig {
    fn env(&self) -> Option<Value> {
        self.config.env()
    }

    fn path(&self) -> Option<Value> {
        self.config.path()
    }

    fn reload(&self) {
        // no-op
    }
}

impl FakeConfig {
    pub fn new(config_file: &Path) -> FakeConfig {
        let config_file = PathBuf::from(config_file);

        let vars = if let Ok(variables) = read(Tag::unknown(), &Some(config_file)) {
            variables
        } else {
            IndexMap::default()
        };

        FakeConfig {
            config: NuConfig {
                vars: Arc::new(Mutex::new(vars)),
            },
        }
    }
}
