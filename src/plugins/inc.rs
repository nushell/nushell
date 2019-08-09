use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, NamedType, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Signature, Tagged, TaggedItem, Value,
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
        value: Tagged<Value>,
        field: &Option<String>,
    ) -> Result<Tagged<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::Int(i)) => Ok(Value::int(i + 1).tagged(value.tag())),
            Value::Primitive(Primitive::Bytes(b)) => {
                Ok(Value::bytes(b + 1 as u64).tagged(value.tag()))
            }
            Value::Primitive(Primitive::String(ref s)) => {
                if let Ok(i) = s.parse::<u64>() {
                    Ok(Tagged::from_item(
                        Value::string(format!("{}", i + 1)),
                        value.tag(),
                    ))
                } else if let Ok(mut ver) = semver::Version::parse(&s) {
                    if self.major {
                        ver.increment_major();
                    } else if self.minor {
                        ver.increment_minor();
                    } else {
                        self.patch;
                        ver.increment_patch();
                    }
                    Ok(Tagged::from_item(
                        Value::string(ver.to_string()),
                        value.tag(),
                    ))
                } else {
                    Err(ShellError::string("string could not be incremented"))
                }
            }
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.tag(), f) {
                        Some(result) => self.inc(result.map(|x| x.clone()), &None)?,
                        None => {
                            return Err(ShellError::string("inc could not find field to replace"))
                        }
                    };
                    match value
                        .item
                        .replace_data_at_path(value.tag(), f, replacement.item.clone())
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
    fn config(&mut self) -> Result<Signature, ShellError> {
        let mut named = IndexMap::new();
        named.insert("major".to_string(), NamedType::Switch);
        named.insert("minor".to_string(), NamedType::Switch);
        named.insert("patch".to_string(), NamedType::Switch);

        Ok(Signature {
            name: "inc".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            named,
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
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
                    Tagged {
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

        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.inc(input, &self.field)?)])
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}
