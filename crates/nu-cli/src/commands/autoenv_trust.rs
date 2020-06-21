use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use directories::ProjectDirs;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::AnchorLocation;
use std::hash::{Hash, Hasher};
use std::io::Write;
use toml_edit::{Document, value};
use std::io::Read;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, OpenOptions},
    path::PathBuf,
};

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

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config_path.clone())?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut toml_table =
            match std::fs::read_to_string(config_path.clone())?.parse::<toml::Value>() {
                Ok(toml_doc) => {
                    let table = match toml_doc.get("allowed-files") {
                        Some(v) => v.clone(),
                        None => r#"[allowed-files]"#.parse::<toml::Value>().unwrap(),
                    };

                    table.as_table().unwrap().clone()
                }
                Err(_) => {
                    let mut table = toml::value::Table::new();
                    table.insert("allowed-files".to_string(), toml::Value::from(""));
                    table

                    // let table = "[allowed-files]".parse::<toml::Value>().unwrap();
                    // table.as_table().unwrap().clone()
                }
            };

        toml_table.insert(
            current_dir.to_string_lossy().to_string(),
            toml::Value::try_from(hasher.finish().to_string())?,
        );
        let toml_string: String = toml::to_string(&toml_table).expect(";");

        fs::write(config_path, toml_string).expect("Couldn't write to toml file");

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
