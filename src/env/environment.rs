use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Environment {
    crate cwd: PathBuf,
}

impl Environment {
    pub fn basic() -> Result<Environment, std::io::Error> {
        let cwd = std::env::current_dir()?;

        Ok(Environment { cwd })
    }

    pub fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }
}
