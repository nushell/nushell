use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::io::Read;

pub struct Autoenv;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Trusted {
    pub files: IndexMap<String, String>,
}
impl Trusted {
    pub fn new() -> Self {
        Trusted {
            files: IndexMap::new(),
        }
    }
    pub fn read_trusted() -> Result<Self, ShellError> {
        let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(config_path.clone())
        {
            Ok(p) => p,
            Err(_) => {
                return Err(ShellError::untagged_runtime_error(
                    "Couldn't open nu-env.toml",
                ));
            }
        };
        let mut doc = String::new();
        file.read_to_string(&mut doc)?;

        let allowed: Trusted = toml::from_str(doc.as_str()).unwrap_or_else(|_| Trusted::new());
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
