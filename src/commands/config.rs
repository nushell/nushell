use crate::errors::ShellError;
use crate::object::config;
use crate::object::Value;
use crate::parser::registry::{NamedType, NamedValue};
use crate::parser::CommandConfig;
use crate::prelude::*;
use indexmap::IndexMap;
use log::trace;

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
        named.insert("set".to_string(), NamedType::Optional(NamedValue::Tuple));
        named.insert("get".to_string(), NamedType::Optional(NamedValue::Single));
        named.insert("clear".to_string(), NamedType::Switch);

        named.insert(
            "remove".to_string(),
            NamedType::Optional(NamedValue::Single),
        );

        CommandConfig {
            name: self.name().to_string(),
            mandatory_positional: vec![],
            optional_positional: vec![],
            rest_positional: false,
            named,
        }
    }
}

pub fn config(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut result = crate::object::config::config()?;

    trace!("{:#?}", args.positional);
    trace!("{:#?}", args.named);

    if let Some(v) = args.named.get("get") {
        let key = v.as_string()?;
        let value = result
            .get(&key)
            .ok_or_else(|| ShellError::string(&format!("Missing key {} in config", key)))?;

        return Ok(
            futures::stream::once(futures::future::ready(ReturnValue::Value(value.clone())))
                .boxed(),
        );
    }

    if let Some(v) = args.named.get("set") {
        if let Ok((key, value)) = v.as_pair() {
            result.insert(key.as_string()?, value.clone());

            config::write_config(&result)?;

            return Ok(
                futures::stream::once(futures::future::ready(ReturnValue::Value(Value::Object(
                    result.into(),
                ))))
                .boxed(),
            );
        }
    }

    if let Some(_) = args.named.get("clear") {
        result.clear();

        config::write_config(&result)?;

        return Ok(
            futures::stream::once(futures::future::ready(ReturnValue::Value(Value::Object(
                result.into(),
            ))))
            .boxed(),
        );
    }

    if let Some(v) = args.named.get("remove") {
        let key = v.as_string()?;

        if result.contains_key(&key) {
            result.remove(&key);
        } else {
            return Err(ShellError::string(&format!(
                "{} does not exist in config",
                key
            )));
        }

        return Ok(
            futures::stream::once(futures::future::ready(ReturnValue::Value(Value::Object(
                result.into(),
            ))))
            .boxed(),
        );
    }

    if args.positional.len() == 0 {
        return Ok(
            futures::stream::once(futures::future::ready(ReturnValue::Value(Value::Object(
                result.into(),
            ))))
            .boxed(),
        );
    }

    Err(ShellError::string(format!("Unimplemented")))
}
