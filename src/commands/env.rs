use crate::cli::History;
use crate::data::config;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{
    Dictionary, Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
    Value,
};
use nu_source::Tagged;

use crate::commands::WholeStreamCommand;
use indexmap::IndexMap;

pub struct Env;

#[derive(Deserialize)]
pub struct EnvArgs {
    set: Option<(Tagged<String>, Value)>,
    get: Option<Tagged<String>>,
    remove: Option<Tagged<String>>,
}

impl WholeStreamCommand for Env {
    fn name(&self) -> &str {
        "env"
    }

    fn signature(&self) -> Signature {
        Signature::build("env")
            .named(
                "set",
                SyntaxShape::Any,
                "set a value in the environment, eg) --set [key value]",
                Some('s'),
            )
            .named(
                "get",
                SyntaxShape::String,
                "get a value from the environment",
                Some('g'),
            )
            .named(
                "remove",
                SyntaxShape::String,
                "remove a value from the environment",
                Some('d'),
            )
    }

    fn usage(&self) -> &str {
        "Get the current environment."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, env)?.run()
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

pub fn env(
    EnvArgs { get, set, remove }: EnvArgs,
    RunnableContext { name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if let Some(key) = get {
        match std::env::var(&key.item) {
            Ok(val) => Ok(OutputStream::one(ReturnSuccess::value(val))),
            Err(err) => Err(ShellError::labeled_error(
                "Variable doesn't exist",
                err.to_string(),
                &key.tag,
            )),
        }
    } else if let Some((key, value)) = set {
        match value.value {
            UntaggedValue::Primitive(Primitive::String(ref s)) => {
                std::env::set_var(&key.item, s);
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "Expected a string",
                    "requires string input",
                    &key.tag,
                ));
            }
        }
        Ok(OutputStream::empty())
    } else if let Some(key) = remove {
        std::env::remove_var(&key.item);
        Ok(OutputStream::empty())
    } else {
        let mut env_out = VecDeque::new();

        let value = get_environment(name)?;
        env_out.push_back(value);

        let env_out = futures::stream::iter(env_out);

        Ok(env_out.to_output_stream())
    }
}
