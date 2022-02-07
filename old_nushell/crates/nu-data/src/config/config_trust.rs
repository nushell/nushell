use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, path::PathBuf};

use indexmap::IndexMap;
use nu_errors::ShellError;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Trusted {
    pub files: IndexMap<String, Vec<u8>>,
}

impl Trusted {
    pub fn new() -> Self {
        Trusted {
            files: IndexMap::new(),
        }
    }
}

pub fn is_file_trusted(nu_env_file: &Path, content: &[u8]) -> Result<bool, ShellError> {
    let contentdigest = Sha256::digest(content).as_slice().to_vec();
    let nufile = nu_path::canonicalize(nu_env_file)?;

    let trusted = read_trusted()?;
    Ok(trusted.files.get(&nufile.to_string_lossy().to_string()) == Some(&contentdigest))
}

pub fn read_trusted() -> Result<Trusted, ShellError> {
    let config_path = crate::config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .open(config_path)
        .map_err(|_| ShellError::untagged_runtime_error("Couldn't open nu-env.toml"))?;
    let mut doc = String::new();
    file.read_to_string(&mut doc)?;

    let allowed = toml::de::from_str(&doc).unwrap_or_else(|_| Trusted::new());
    Ok(allowed)
}
