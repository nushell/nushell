use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::config;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use std::path::PathBuf;

pub struct Config;

#[derive(Deserialize)]
pub struct ConfigArgs {
    load: Option<Tagged<PathBuf>>,
    set: Option<(Tagged<String>, Value)>,
    set_into: Option<Tagged<String>>,
    get: Option<Tagged<String>>,
    clear: Tagged<bool>,
    remove: Option<Tagged<String>>,
    path: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for Config {
    fn name(&self) -> &str {
        "config"
    }

    fn signature(&self) -> Signature {
        Signature::build("config")
            .named(
                "load",
                SyntaxShape::Path,
                "load the config from the path given",
                Some('l'),
            )
            .named(
                "set",
                SyntaxShape::Any,
                "set a value in the config, eg) --set [key value]",
                Some('s'),
            )
            .named(
                "set_into",
                SyntaxShape::String,
                "sets a variable from values in the pipeline",
                Some('i'),
            )
            .named(
                "get",
                SyntaxShape::Any,
                "get a value from the config",
                Some('g'),
            )
            .named(
                "remove",
                SyntaxShape::Any,
                "remove a value from the config",
                Some('r'),
            )
            .switch("clear", "clear the config", Some('c'))
            .switch("path", "return the path to the config file", Some('p'))
    }

    fn usage(&self) -> &str {
        "Configuration management."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        config(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "See all config values",
                example: "config",
                result: None,
            },
            Example {
                description: "Set completion_mode to circular",
                example: "config --set [completion_mode circular]",
                result: None,
            },
            Example {
                description: "Store the contents of the pipeline as a path",
                example: "echo ['/usr/bin' '/bin'] | config --set_into path",
                result: None,
            },
            Example {
                description: "Get the current startup commands",
                example: "config --get startup",
                result: None,
            },
            Example {
                description: "Remove the startup commands",
                example: "config --remove startup",
                result: None,
            },
            Example {
                description: "Clear the config (be careful!)",
                example: "config --clear",
                result: None,
            },
            Example {
                description: "Get the path to the current config file",
                example: "config --path",
                result: None,
            },
        ]
    }
}

pub async fn config(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.clone();
    let name = args.call_info.name_tag.clone();
    let registry = registry.clone();

    let (
        ConfigArgs {
            load,
            set,
            set_into,
            get,
            clear,
            remove,
            path,
        },
        input,
    ) = args.process(&registry).await?;

    let configuration = if let Some(supplied) = load {
        Some(supplied.item().clone())
    } else {
        None
    };

    let mut result = crate::data::config::read(name_span, &configuration)?;

    Ok(if let Some(v) = get {
        let key = v.to_string();
        let value = result
            .get(&key)
            .ok_or_else(|| ShellError::labeled_error("Missing key in config", "key", v.tag()))?;

        match value {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => {
                let list: Vec<_> = list
                    .iter()
                    .map(|x| ReturnSuccess::value(x.clone()))
                    .collect();

                futures::stream::iter(list).to_output_stream()
            }
            x => {
                let x = x.clone();
                OutputStream::one(ReturnSuccess::value(x))
            }
        }
    } else if let Some((key, value)) = set {
        result.insert(key.to_string(), value.clone());

        config::write(&result, &configuration)?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(&value.tag),
        ))
    } else if let Some(v) = set_into {
        let rows: Vec<Value> = input.collect().await;
        let key = v.to_string();

        if rows.is_empty() {
            return Err(ShellError::labeled_error(
                "No values given for set_into",
                "needs value(s) from pipeline",
                v.tag(),
            ));
        } else if rows.len() == 1 {
            // A single value
            let value = &rows[0];

            result.insert(key, value.clone());

            config::write(&result, &configuration)?;

            OutputStream::one(ReturnSuccess::value(
                UntaggedValue::Row(result.into()).into_value(name),
            ))
        } else {
            // Take in the pipeline as a table
            let value = UntaggedValue::Table(rows).into_value(name.clone());

            result.insert(key, value);

            config::write(&result, &configuration)?;

            OutputStream::one(ReturnSuccess::value(
                UntaggedValue::Row(result.into()).into_value(name),
            ))
        }
    } else if let Tagged { item: true, tag } = clear {
        result.clear();

        config::write(&result, &configuration)?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(tag),
        ))
    } else if let Tagged { item: true, tag } = path {
        let path = config::default_path_for(&configuration)?;

        OutputStream::one(ReturnSuccess::value(
            UntaggedValue::Primitive(Primitive::Path(path)).into_value(tag),
        ))
    } else if let Some(v) = remove {
        let key = v.to_string();

        if result.contains_key(&key) {
            result.swap_remove(&key);
            config::write(&result, &configuration)?;
            futures::stream::iter(vec![ReturnSuccess::value(
                UntaggedValue::Row(result.into()).into_value(v.tag()),
            )])
            .to_output_stream()
        } else {
            return Err(ShellError::labeled_error(
                "Key does not exist in config",
                "key",
                v.tag(),
            ));
        }
    } else {
        futures::stream::iter(vec![ReturnSuccess::value(
            UntaggedValue::Row(result.into()).into_value(name),
        )])
        .to_output_stream()
    })
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Config {})
    }
}
