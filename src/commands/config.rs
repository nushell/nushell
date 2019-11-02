use crate::commands::WholeStreamCommand;
use crate::data::{config, Value};
use crate::errors::ShellError;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry::{self};
use crate::prelude::*;
use std::path::PathBuf;

pub struct Config;

#[derive(Deserialize)]
pub struct ConfigArgs {
    load: Option<Tagged<PathBuf>>,
    set: Option<(Tagged<String>, Tagged<Value>)>,
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
                "load the config from the path give",
            )
            .named("set", SyntaxShape::Any, "set a value in the config")
            .named("get", SyntaxShape::Any, "get a value from the config")
            .named("remove", SyntaxShape::Any, "remove a value from the config")
            .switch("clear", "clear the config")
            .switch("path", "return the path to the config file")
    }

    fn usage(&self) -> &str {
        "Configuration management."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, config)?.run()
    }
}

pub fn config(
    ConfigArgs {
        load,
        set,
        get,
        clear,
        remove,
        path,
    }: ConfigArgs,
    RunnableContext { name, .. }: RunnableContext,
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
                Tagged {
                    item: Value::Table(list),
                    ..
                } => {
                    for l in list {
                        yield ReturnSuccess::value(l.clone());
                    }
                }
                x => yield ReturnSuccess::value(x.clone()),
            }
        }
        else if let Some((key, value)) = set {
            result.insert(key.to_string(), value.clone());

            config::write(&result, &configuration)?;

            yield ReturnSuccess::value(Value::Row(result.into()).tagged(value.tag()));
        }
        else if let Tagged { item: true, tag } = clear {
            result.clear();

            config::write(&result, &configuration)?;

            yield ReturnSuccess::value(Value::Row(result.into()).tagged(tag));

            return;
        }
        else if let Tagged { item: true, tag } = path {
            let path = config::default_path_for(&configuration)?;

            yield ReturnSuccess::value(Value::Primitive(Primitive::Path(path)).tagged(tag));
        }
        else if let Some(v) = remove {
            let key = v.to_string();

            if result.contains_key(&key) {
                result.swap_remove(&key);
                config::write(&result, &configuration).unwrap();
            } else {
                yield Err(ShellError::labeled_error(
                    "Key does not exist in config",
                    "key",
                    v.tag(),
                ));
            }

            yield ReturnSuccess::value(Value::Row(result.into()).tagged(v.tag()));
        }
        else {
            yield ReturnSuccess::value(Value::Row(result.into()).tagged(name));
        }
    };

    Ok(stream.to_output_stream())
}
