use super::autoenv::read_trusted;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::SyntaxShape;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub struct AutoenvTrust;

#[async_trait]
impl WholeStreamCommand for AutoenvTrust {
    fn name(&self) -> &str {
        "autoenv trust"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv trust").optional("dir", SyntaxShape::String, "Directory to allow")
    }

    fn usage(&self) -> &str {
        "Trust a .nu-env file in the current or given directory"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();

        let file_to_trust = match args.call_info.evaluate(registry).await?.args.nth(0) {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(ref path)),
                tag: _,
            }) => {
                let mut dir = fs::canonicalize(path)?;
                dir.push(".nu-env");
                dir
            }
            _ => {
                let mut dir = fs::canonicalize(std::env::current_dir()?)?;
                dir.push(".nu-env");
                dir
            }
        };

        let content = std::fs::read(&file_to_trust)?;

        let filename = file_to_trust.to_string_lossy().to_string();
        let mut allowed = read_trusted()?;
        allowed
            .files
            .insert(filename, Sha256::digest(&content).as_slice().to_vec());

        let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;
        let tomlstr = toml::to_string(&allowed).map_err(|_| {
            ShellError::untagged_runtime_error("Couldn't serialize allowed dirs to nu-env.toml")
        })?;
        fs::write(config_path, tomlstr).expect("Couldn't write to toml file");

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(".nu-env trusted!").into_value(tag),
        )))
    }
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Allow .nu-env file in current directory",
                example: "autoenv trust",
                result: None,
            },
            Example {
                description: "Allow .nu-env file in directory foo",
                example: "autoenv trust foo",
                result: None,
            },
        ]
    }
}
