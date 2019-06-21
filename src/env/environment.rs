use crate::object::base::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Environment {
    crate obj: Value,
    crate path: PathBuf,
}

impl Environment {
    pub fn basic() -> Result<Environment, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(Environment {
            obj: Value::Filesystem,
            path,
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn obj(&self) -> &Value {
        &self.obj
    }
}
