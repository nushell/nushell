use indexmap::IndexMap;
use nu::{
    serve_plugin, Args, CommandConfig, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Spanned, SpannedItem, Value,
};

struct Inc {
    field: Option<String>,
}
impl Inc {
    fn new() -> Inc {
        Inc { field: None }
    }

    fn inc(value: Spanned<Value>, field: &Option<String>) -> Result<Spanned<Value>, ShellError> {
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
                } else {
                    Err(ShellError::string("string could not be incremented"))
                }
            }
            Value::Object(_) => match field {
                Some(f) => {
                    let replacement = match value.item.get_data_by_path(value.span, f) {
                        Some(result) => Inc::inc(result.map(|x| x.clone()), &None)?,
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
        Ok(CommandConfig {
            name: "inc".to_string(),
            positional: vec![PositionalType::optional_any("Field")],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, args: Args) -> Result<(), ShellError> {
        if let Some(args) = args.positional {
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
        Ok(vec![ReturnSuccess::value(Inc::inc(input, &self.field)?)])
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}
