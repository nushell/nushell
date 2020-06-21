use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::{collections::hash_map::DefaultHasher, fs, path::PathBuf};
use serde_derive::Serialize;

pub struct AutoenvTrust;

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

        let mut toml_doc = doc.parse::<toml::Value>().unwrap();
        let mut empty = toml::value::Value::Table(toml::value::Table::new());
        toml_doc
            .get_mut("allowed-files")
            .unwrap_or_else(|| &mut empty)
            .as_table_mut()
            .unwrap()
            .insert(
                current_dir.to_string_lossy().to_string(),
                toml::Value::try_from(hasher.finish().to_string())?,
            );

        fs::write(config_path, toml_doc.as_str().unwrap()).expect("Couldn't write to toml file");

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
