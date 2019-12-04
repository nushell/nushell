use crate::cli::History;
use crate::data::config;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, TaggedDictBuilder, UntaggedValue, Value};

use crate::commands::WholeStreamCommand;
use indexmap::IndexMap;

pub struct Env;

impl WholeStreamCommand for Env {
    fn name(&self) -> &str {
        "env"
    }

    fn signature(&self) -> Signature {
        Signature::build("env")
    }

    fn usage(&self) -> &str {
        "Get the current environment."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        env(args, registry)
    }
}

pub fn get_environment(tag: Tag) -> Result<Value, Box<dyn std::error::Error>> {
    let mut indexmap = IndexMap::new();

    let path = std::env::current_dir()?;
    indexmap.insert(
        "cwd".to_string(),
        UntaggedValue::path(path).into_value(&tag),
    );

    if let Some(home) = dirs::home_dir() {
        indexmap.insert(
            "home".to_string(),
            UntaggedValue::path(home).into_value(&tag),
        );
    }

    let config = config::default_path()?;
    indexmap.insert(
        "config".to_string(),
        UntaggedValue::path(config).into_value(&tag),
    );

    let history = History::path();
    indexmap.insert(
        "history".to_string(),
        UntaggedValue::path(history).into_value(&tag),
    );

    let temp = std::env::temp_dir();
    indexmap.insert(
        "temp".to_string(),
        UntaggedValue::path(temp).into_value(&tag),
    );

    let mut dict = TaggedDictBuilder::new(&tag);
    for v in std::env::vars() {
        dict.insert_untagged(v.0, UntaggedValue::string(v.1));
    }
    if !dict.is_empty() {
        indexmap.insert("vars".to_string(), dict.into_value());
    }

    Ok(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(&tag))
}

pub fn env(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let mut env_out = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let value = get_environment(tag)?;
    env_out.push_back(value);

    Ok(env_out.to_output_stream())
}
