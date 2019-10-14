use crate::commands::WholeStreamCommand;
use crate::data::{config, Value};
use crate::errors::ShellError;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry::{self};
use crate::prelude::*;
use std::iter::FromIterator;
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
            .named("load", SyntaxShape::Path)
            .named("set", SyntaxShape::Any)
            .named("get", SyntaxShape::Any)
            .named("remove", SyntaxShape::Any)
            .switch("clear")
            .switch("path")
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

    let configuration = if let Some(supplied) = load {
        Some(supplied.item().clone())
    } else {
        None
    };

    let mut result = crate::data::config::read(name_span, &configuration)?;

    if let Some(v) = get {
        let key = v.to_string();
        let value = result.get(&key).ok_or_else(|| {
            ShellError::labeled_error(&format!("Missing key in config"), "key", v.tag())
        })?;

        let mut results = VecDeque::new();

        match value {
            Tagged {
                item: Value::Table(list),
                ..
            } => {
                for l in list {
                    results.push_back(ReturnSuccess::value(l.clone()));
                }
            }
            x => results.push_back(ReturnSuccess::value(x.clone())),
        }

        return Ok(results.to_output_stream());
    }

    if let Some((key, value)) = set {
        result.insert(key.to_string(), value.clone());

        config::write(&result, &configuration)?;

        return Ok(stream![Value::Row(result.into()).tagged(value.tag())].from_input_stream());
    }

    if let Tagged { item: true, tag } = clear {
        result.clear();

        config::write(&result, &configuration)?;

        return Ok(stream![Value::Row(result.into()).tagged(tag)].from_input_stream());
    }

    if let Tagged { item: true, tag } = path {
        let path = config::default_path_for(&configuration)?;

        return Ok(stream![Value::Primitive(Primitive::Path(path)).tagged(tag)].from_input_stream());
    }

    if let Some(v) = remove {
        let key = v.to_string();

        if result.contains_key(&key) {
            result.swap_remove(&key);
            config::write(&result, &configuration)?;
        } else {
            return Err(ShellError::labeled_error(
                "{} does not exist in config",
                "key",
                v.tag(),
            ));
        }

        let obj = VecDeque::from_iter(vec![Value::Row(result.into()).tagged(v.tag())]);
        return Ok(obj.from_input_stream());
    }

    return Ok(vec![Value::Row(result.into()).tagged(name)].into());
}
