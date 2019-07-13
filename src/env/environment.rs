use crate::object::base::Value;
use crate::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Environment {
    crate obj: Spanned<Value>,
    crate path: PathBuf,
}

impl Environment {
    pub fn basic() -> Result<Environment, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(Environment {
            obj: Value::Filesystem.spanned_unknown(),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn obj(&self) -> &Spanned<Value> {
        &self.obj
    }
}
