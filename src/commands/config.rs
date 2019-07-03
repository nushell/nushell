#[macro_use]
use crate::prelude::*;

use crate::errors::ShellError;
use crate::object::config;
use crate::object::Value;
use crate::parser::registry::{CommandConfig, NamedType, NamedValue};
use indexmap::IndexMap;
use log::trace;
use std::iter::FromIterator;

pub struct Config;

impl Command for Config {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        config(args)
    }
    fn name(&self) -> &str {
        "config"
    }

    fn config(&self) -> CommandConfig {
        let mut named: IndexMap<String, NamedType> = IndexMap::new();
        named.insert("set".to_string(), NamedType::Optional(NamedValue::Single));
        named.insert("get".to_string(), NamedType::Optional(NamedValue::Single));
        named.insert("clear".to_string(), NamedType::Switch);

        named.insert(
            "remove".to_string(),
            NamedType::Optional(NamedValue::Single),
        );

        CommandConfig {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: false,
            named,
            is_sink: true,
            is_filter: false,
            can_load: vec![],
            can_save: vec![],
        }
    }
}

pub fn config(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut result = crate::object::config::config()?;

    trace!("{:#?}", args.args.positional);
    trace!("{:#?}", args.args.named);

    if let Some(v) = args.get("get") {
        let key = v.as_string()?;
        let value = result
            .get(&key)
            .ok_or_else(|| ShellError::string(&format!("Missing key {} in config", key)))?;

        return Ok(
            vec![value.clone()].into(), // futures::stream::once(futures::future::ready(ReturnSuccess::Value(value.clone()))).into(),
        );
    }

    if let Some(v) = args.get("set") {
        if let Ok((key, value)) = v.as_pair() {
            result.insert(key.as_string()?.to_string(), value.clone());

            config::write_config(&result)?;

            return Ok(stream![Value::Object(result.into())].from_input_stream());
        }
    }

    if let Some(_) = args.get("clear") {
        result.clear();

        config::write_config(&result)?;

        return Ok(stream![Value::Object(result.into())].from_input_stream());
    }

    if let Some(v) = args.get("remove") {
        let key = v.as_string()?;

        if result.contains_key(&key) {
            result.remove(&key);
        } else {
            return Err(ShellError::string(&format!(
                "{} does not exist in config",
                key
            )));
        }

        let obj = VecDeque::from_iter(vec![Value::Object(result.into())]);
        return Ok(obj.from_input_stream());
    }

    if args.len() == 0 {
        return Ok(vec![Value::Object(result.into())].into());
    }

    Err(ShellError::string(format!("Unimplemented")))
}
