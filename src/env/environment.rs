use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Environment {
    crate path: PathBuf,
}

impl Environment {
    pub fn basic() -> Result<Environment, std::io::Error> {
        let path = std::env::current_dir()?;

        Ok(Environment { path })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
