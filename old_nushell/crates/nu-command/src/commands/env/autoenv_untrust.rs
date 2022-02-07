use crate::prelude::*;
use nu_data::config::Trusted;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::SyntaxShape;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use std::io::Read;
use std::{fs, path::PathBuf};
pub struct AutoenvUntrust;

impl WholeStreamCommand for AutoenvUntrust {
    fn name(&self) -> &str {
        "autoenv untrust"
    }

    fn signature(&self) -> Signature {
        Signature::build("autoenv untrust")
            .optional("dir", SyntaxShape::String, "Directory to disallow")
            .switch("quiet", "Don't output success message", Some('q'))
    }

    fn usage(&self) -> &str {
        "Untrust a .nu-env file in the current or given directory"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let file_to_untrust = match args.opt(0)? {
            Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(ref path)),
                tag: _,
            }) => {
                let mut dir = nu_path::canonicalize(path)?;
                dir.push(".nu-env");
                dir
            }
            _ => {
                let mut dir = std::env::current_dir()?;
                dir.push(".nu-env");
                dir
            }
        };
        let quiet = args.has_flag("quiet");

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

        let mut allowed: Trusted = toml::from_str(&doc).unwrap_or_else(|_| Trusted::new());

        let file_to_untrust = file_to_untrust.to_string_lossy().to_string();

        if allowed.files.remove(&file_to_untrust).is_none() {
            return
                Err(ShellError::untagged_runtime_error(
                    "No .nu-env file to untrust in the given directory. Is it missing, or already untrusted?",
                ));
        }

        let tomlstr = toml::to_string(&allowed).map_err(|_| {
            ShellError::untagged_runtime_error("Couldn't serialize allowed dirs to nu-env.toml")
        })?;
        fs::write(config_path, tomlstr).expect("Couldn't write to toml file");

        if quiet {
            Ok(ActionStream::empty())
        } else {
            Ok(ActionStream::one(ReturnSuccess::value(
                UntaggedValue::string(".nu-env untrusted!").into_value(tag),
            )))
        }
    }
    fn is_binary(&self) -> bool {
        false
    }
    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Disallow .nu-env file in current directory",
                example: "autoenv untrust",
                result: None,
            },
            Example {
                description: "Disallow .nu-env file in directory foo",
                example: "autoenv untrust foo",
                result: None,
            },
        ]
    }
}
