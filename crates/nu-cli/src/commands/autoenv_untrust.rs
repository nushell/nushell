use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::SyntaxShape;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use std::io::{Read, Write};
use std::{fs, path::PathBuf};
use super::autoenv::Allowed;
pub struct AutoenvUnTrust;


#[async_trait]
impl WholeStreamCommand for AutoenvUnTrust {
    fn name(&self) -> &str {
        "autoenv untrust"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv untrust").optional("dir", SyntaxShape::String, "Directory to disallow")
    }

    fn usage(&self) -> &str {
        "Untrust a .nu-env file in the current or given directory"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {

        let tag = args.call_info.name_tag.clone();
        let dir_to_untrust = match args.call_info.evaluate(registry).await?.args.nth(0) {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(ref path)),
                tag: _,
            }) => PathBuf::from(path),
            _ => std::env::current_dir()?,
        };

        let config_path = config::default_path_for(&Some(PathBuf::from("nu-env.toml")))?;

        let mut file = match std::fs::OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(config_path.clone()) {
                Ok(p) => p,
                Err(_) => {
                    return Err(ShellError::untagged_runtime_error("Couldn't open nu-env.toml"));
                }
            };

        let mut doc = String::new();
        file.read_to_string(&mut doc)?;

        let mut allowed: Allowed = toml::from_str(doc.as_str()).unwrap_or_else(|_| Allowed {
            dirs: IndexMap::new(),
        });
        allowed.dirs.remove(&dir_to_untrust);

        fs::write(config_path, toml::to_string(&allowed).unwrap())
            .expect("Couldn't write to toml file");

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(".nu-env untrusted!").into_value(tag),
        )))
    }
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}