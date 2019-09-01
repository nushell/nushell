use crate::prelude::*;

use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::object::{config, Value};
use crate::parser::hir::SyntaxType;
use crate::parser::registry::{self};
use std::iter::FromIterator;

pub struct Config;

#[derive(Deserialize)]
pub struct ConfigArgs {
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
            .named("set", SyntaxType::Any)
            .named("get", SyntaxType::Any)
            .named("remove", SyntaxType::Any)
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
        set,
        get,
        clear,
        remove,
        path,
    }: ConfigArgs,
    RunnableContext { name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let mut result = crate::object::config::config(name)?;

    if let Some(v) = get {
        let key = v.to_string();
        let value = result
            .get(&key)
            .ok_or_else(|| ShellError::string(&format!("Missing key {} in config", key)))?;

        return Ok(
            stream![value.clone()].into(), // futures::stream::once(futures::future::ready(ReturnSuccess::Value(value.clone()))).into(),
        );
    }

    if let Some((key, value)) = set {
        result.insert(key.to_string(), value.clone());

        config::write_config(&result)?;

        return Ok(stream![Tagged::from_simple_spanned_item(
            Value::Object(result.into()),
            value.span()
        )]
        .from_input_stream());
    }

    if let Tagged {
        item: true,
        tag: Tag { span, .. },
    } = clear
    {
        result.clear();

        config::write_config(&result)?;

        return Ok(stream![Tagged::from_simple_spanned_item(
            Value::Object(result.into()),
            span
        )]
        .from_input_stream());
    }

    if let Tagged {
        item: true,
        tag: Tag { span, .. },
    } = path
    {
        let path = config::config_path()?;

        return Ok(stream![Tagged::from_simple_spanned_item(
            Value::Primitive(Primitive::Path(path)),
            span
        )]
        .from_input_stream());
    }

    if let Some(v) = remove {
        let key = v.to_string();

        if result.contains_key(&key) {
            result.remove(&key);
            config::write_config(&result)?;
        } else {
            return Err(ShellError::string(&format!(
                "{} does not exist in config",
                key
            )));
        }

        let obj = VecDeque::from_iter(vec![Value::Object(result.into()).simple_spanned(v.span())]);
        return Ok(obj.from_input_stream());
    }

    return Ok(vec![Value::Object(result.into()).simple_spanned(name)].into());
}
