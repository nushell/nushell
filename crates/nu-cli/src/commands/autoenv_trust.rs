use super::{autoenv::Allowed, cd::CdArgs};
use crate::commands::WholeStreamCommand;
use crate::{path, prelude::*};
use nu_errors::ShellError;
use nu_protocol::SyntaxShape;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::{collections::hash_map::DefaultHasher, fs, path::PathBuf};
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
                let mut dir = path::absolutize(std::env::current_dir()?, path);
                dir.push(".nu-env");
                dir
            }
            _ => {
                let mut dir = std::env::current_dir()?;
                dir.push(".nu-env");
                dir
            }
        };

        let content = std::fs::read_to_string(&file_to_trust)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);

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

        let mut allowed: Allowed = toml::from_str(doc.as_str()).unwrap_or_else(|_| Allowed::new());

        let file_to_untrust = file_to_trust.to_string_lossy().to_string();
        allowed
            .files
            .insert(file_to_untrust, hasher.finish().to_string());

        fs::write(config_path, toml::to_string(&allowed).unwrap())
            .expect("Couldn't write to toml file");

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
