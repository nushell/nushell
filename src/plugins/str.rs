use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, NamedType, Plugin, PositionalType, Primitive,
    ReturnSuccess, ReturnValue, ShellError, Spanned, SpannedItem, Value,
};

struct Str {
    field: Option<String>,
    downcase: bool,
    upcase: bool,
}

impl Str {
    fn new() -> Str {
        Str {
            field: None,
            downcase: false,
            upcase: false,
        }
    }

    fn strutils(
        &self,
        value: Spanned<Value>,
        field: &Option<String>,
    ) -> Result<Spanned<Value>, ShellError> {
        match value.item {
            Value::Primitive(Primitive::String(s)) => {
                let applied = if self.downcase {
                    Value::string(s.to_ascii_lowercase())
                } else if self.upcase {
                    Value::string(s.to_ascii_uppercase())
                } else {
                    Value::string(s)
                };

                Ok(Spanned {
                    item: applied,
                    span: value.span,
                })
            }
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.span, f) {
                        Some(result) => self.strutils(result.map(|x| x.clone()), &None)?,
                        None => {
                            return Err(ShellError::string("str could not find field to replace"))
                        }
                    };
                    match value
                        .item
                        .replace_data_at_path(value.span, f, replacement.item.clone())
                    {
                        Some(v) => return Ok(v),
                        None => {
                            return Err(ShellError::string("str could not find field to replace"))
                        }
                    }
                }
                None => Err(ShellError::string(
                    "str needs a field when applying it to a value in an object",
                )),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Str {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        let mut named = IndexMap::new();
        named.insert("downcase".to_string(), NamedType::Switch);
        named.insert("upcase".to_string(), NamedType::Switch);

        Ok(CommandConfig {
            name: "str".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            is_sink: false,
            named,
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if call_info.args.has("downcase") {
            self.downcase = true;
        } else if call_info.args.has("upcase") {
            self.upcase = true;
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

        Ok(vec![])
    }

    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(
            self.strutils(input, &self.field)?,
        )])
    }
}

fn main() {
    serve_plugin(&mut Str::new());
}
