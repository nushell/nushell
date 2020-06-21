use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use serde::Deserialize;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::{collections::hash_map::DefaultHasher, fs, path::PathBuf};
pub struct AutoenvTrust;

#[derive(Deserialize, Serialize)]
struct Allowed {
    pub dirs: IndexMap<String, String>,
}

#[async_trait]
impl WholeStreamCommand for AutoenvTrust {
    fn name(&self) -> &str {
        "autoenv trust"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv trust")
    }

    fn usage(&self) -> &str {
        "Trust a .nu-env file in the current directory"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let current_dir = std::env::current_dir()?;
        let content = std::fs::read_to_string(current_dir.join(".nu-env"))?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);

        let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(config_path.clone())
            .unwrap();
        let mut doc = String::new();
        file.read_to_string(&mut doc)?;

        let mut allowed: Allowed = toml::from_str(doc.as_str()).unwrap_or_else(|_| Allowed {
            dirs: IndexMap::new(),
        });
        allowed.dirs.insert(
            current_dir.to_string_lossy().to_string(),
            hasher.finish().to_string(),
        );

        fs::write(config_path, toml::to_string(&allowed).unwrap())
            .expect("Couldn't write to toml file");

        let tag = args.call_info.name_tag.clone();
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(".nu-env trusted!").into_value(tag),
        )))
    }
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}
