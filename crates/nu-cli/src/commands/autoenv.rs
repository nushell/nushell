use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;
pub struct Autoenv;

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
    pub fn file_is_trusted_reload_config(
        &mut self,
        nu_env_file: &PathBuf,
        content: &Vec<u8>,
    ) -> Result<bool, ShellError> {
        let contentdigest = Sha256::digest(&content).as_slice().to_vec();
        let nufile = nu_env_file.to_str().unwrap_or("");

        //If the existing hash for nu_env_file exists and corresponds to the hash of content, everything is fine.
        if self.files.get(nufile) == Some(&contentdigest) {
            return Ok(true);
        }
        //Otherwise, we re-read nu-env.toml and compare with that. If it is the same, we update the map.
        //This allows the user to use autoenv trust without having to restart nushell afterwards
        let new_trusted = Trusted::read_trusted()?;
        if new_trusted.files.get(nufile) == Some(&contentdigest) {
            self.files = new_trusted.files;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn read_trusted() -> Result<Self, ShellError> {
        let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(config_path)
            .or_else(|_| {
                Err(ShellError::untagged_runtime_error(
                    "Couldn't open nu-env.toml",
                ))
            })?;
        let mut doc = String::new();
        file.read_to_string(&mut doc)?;

        let allowed = toml::de::from_str(doc.as_str()).unwrap_or_else(|_| Trusted::new());

        Ok(allowed)
    }
}


#[async_trait]
impl WholeStreamCommand for Autoenv {
    fn name(&self) -> &str {
        "autoenv"
    }
    fn usage(&self) -> &str {
        // "Mark a .nu-env file in a directory as trusted. Needs to be re-run after each change to the file or its filepath."
        "Manage directory specific environments"
    }
    fn signature(&self) -> Signature {
        Signature::build("autoenv")
    }
    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(crate::commands::help::get_help(&Autoenv, &registry))
                .into_value(Tag::unknown()),
        )))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Allow .nu-env file in current directory",
            example: "autoenv trust",
            result: None,
        }]
    }
}
