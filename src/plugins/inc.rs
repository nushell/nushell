use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, NamedType, Plugin, PositionalType, Primitive,
    ReturnSuccess, ReturnValue, ShellError, Spanned, SpannedItem, Value,
};

struct Inc {
    field: Option<String>,
    major: bool,
    minor: bool,
    patch: bool,
}
impl Inc {
    fn new() -> Inc {
        Inc {
            field: None,
            major: false,
            minor: false,
            patch: false,
        }
    }

    fn inc(
        &self,
        value: Spanned<Value>,
        field: &Option<String>,
    ) -> Result<Spanned<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::Int(i)) => Ok(Value::int(i + 1).spanned(value.span)),
            Value::Primitive(Primitive::Bytes(b)) => {
                Ok(Value::bytes(b + 1 as u64).spanned(value.span))
            }
            Value::Primitive(Primitive::String(s)) => {
                if let Ok(i) = s.parse::<u64>() {
                    Ok(Spanned {
                        item: Value::string(format!("{}", i + 1)),
                        span: value.span,
                    })
                } else if let Ok(mut ver) = semver::Version::parse(&s) {
                    if self.major {
                        ver.increment_major();
                    } else if self.minor {
                        ver.increment_minor();
                    } else {
                        self.patch;
                        ver.increment_patch();
                    }
                    Ok(Spanned {
                        item: Value::string(ver.to_string()),
                        span: value.span,
                    })
                } else {
                    Err(ShellError::string("string could not be incremented"))
                }
            }
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.span, f) {
                        Some(result) => self.inc(result.map(|x| x.clone()), &None)?,
                        None => {
                            return Err(ShellError::string("inc could not find field to replace"))
                        }
                    };
                    match value
                        .item
                        .replace_data_at_path(value.span, f, replacement.item.clone())
                    {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::string("inc could not find field to replace"))
                        }
                    }
                }
                None => Err(ShellError::string(
                    "inc needs a field when incrementing a value in an object",
                )),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Inc {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        let mut named = IndexMap::new();
        named.insert("major".to_string(), NamedType::Switch);
        named.insert("minor".to_string(), NamedType::Switch);
        named.insert("patch".to_string(), NamedType::Switch);

        Ok(CommandConfig {
            name: "inc".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            is_sink: false,
            named,
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<(), ShellError> {
        if call_info.args.has("major") {
            self.major = true;
        }
        if call_info.args.has("minor") {
            self.minor = true;
        }
        if call_info.args.has("patch") {
            self.patch = true;
        }

        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    Spanned {
                        item: Value::Primitive(Primitive::String(s)),
                        ..
                    } => {
                        self.field = Some(s);
                    }
                    _ => {
                        return Err(ShellError::string(format!(
                            "Unrecognized type in params: {:?}",
                            arg
                        )))
                    }
                }
            }
        }

        Ok(())
    }

    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.inc(input, &self.field)?)])
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}
