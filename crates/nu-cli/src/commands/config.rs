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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, config)?.run()
    }

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "See all config values",
                example: "config",
            },
            Example {
                description: "Set completion_mode to circular",
                example: "config --set [completion_mode circular]",
            },
            Example {
                description: "Store the contents of the pipeline as a path",
                example: "echo ['/usr/bin' '/bin'] | config --set_into path",
            },
            Example {
                description: "Get the current startup commands",
                example: "config --get startup",
            },
            Example {
                description: "Remove the startup commands",
                example: "config --remove startup",
            },
            Example {
                description: "Clear the config (be careful!)",
                example: "config --clear",
            },
            Example {
                description: "Get the path to the current config file",
                example: "config --path",
            },
        ]
    }
}

pub fn config(
    ConfigArgs {
        load,
        set,
        set_into,
        get,
        clear,
        remove,
        path,
    }: ConfigArgs,
    RunnableContext { name, input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let name_span = name.clone();

    let stream = async_stream! {
        let configuration = if let Some(supplied) = load {
            Some(supplied.item().clone())
        } else {
            None
        };

        let mut result = crate::data::config::read(name_span, &configuration)?;

        if let Some(v) = get {
            let key = v.to_string();
            let value = result
                .get(&key)
                .ok_or_else(|| ShellError::labeled_error("Missing key in config", "key", v.tag()))?;

            match value {
                Value {
                    value: UntaggedValue::Table(list),
                    ..
                } => {
                    for l in list {
                        let value = l.clone();
                        yield ReturnSuccess::value(l.clone());
                    }
                }
                x => yield ReturnSuccess::value(x.clone()),
            }
        }
        else if let Some((key, value)) = set {
            result.insert(key.to_string(), value.clone());

            config::write(&result, &configuration)?;

            yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(&value.tag));
        }
        else if let Some(v) = set_into {
            let rows: Vec<Value> = input.collect().await;
            let key = v.to_string();

            if rows.len() == 0 {
                yield Err(ShellError::labeled_error("No values given for set_into", "needs value(s) from pipeline", v.tag()));
            } else if rows.len() == 1 {
                // A single value
                let value = &rows[0];

                result.insert(key.to_string(), value.clone());

                config::write(&result, &configuration)?;

                yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(name));
            } else {
                // Take in the pipeline as a table
                let value = UntaggedValue::Table(rows).into_value(name.clone());

                result.insert(key.to_string(), value.clone());

                config::write(&result, &configuration)?;

                yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(name));
            }
        }
        else if let Tagged { item: true, tag } = clear {
            result.clear();

            config::write(&result, &configuration)?;

            yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(tag));

            return;
        }
        else if let Tagged { item: true, tag } = path {
            let path = config::default_path_for(&configuration)?;

            yield ReturnSuccess::value(UntaggedValue::Primitive(Primitive::Path(path)).into_value(tag));
        }
        else if let Some(v) = remove {
            let key = v.to_string();

            if result.contains_key(&key) {
                result.swap_remove(&key);
                config::write(&result, &configuration)?
            } else {
                yield Err(ShellError::labeled_error(
                    "Key does not exist in config",
                    "key",
                    v.tag(),
                ));
            }

            yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(v.tag()));
        }
        else {
            yield ReturnSuccess::value(UntaggedValue::Row(result.into()).into_value(name));
        }
    };

    Ok(stream.to_output_stream())
}
