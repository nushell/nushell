use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Spanned, Value,
};

struct Edit {
    field: Option<String>,
    value: Option<Value>,
}
impl Edit {
    fn new() -> Edit {
        Edit {
            field: None,
            value: None,
        }
    }

    fn edit(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
        match (value.item, self.value.clone()) {
            (obj @ Value::Object(_), Some(v)) => match &self.field {
                Some(f) => match obj.replace_data_at_path(value.span, &f, v) {
                    Some(v) => return Ok(v),
                    None => {
                        return Err(ShellError::string(
                            "edit could not find place to insert field",
                        ))
                    }
                },
                None => Err(ShellError::string(
                    "edit needs a field when adding a value to an object",
                )),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Edit {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "edit".to_string(),
            positional: vec![
                PositionalType::mandatory_any("Field"),
                PositionalType::mandatory_any("Value"),
            ],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<(), ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Spanned {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.field = Some(s.clone());
                }
                _ => {
                    return Err(ShellError::string(format!(
                        "Unrecognized type in params: {:?}",
                        args[0]
                    )))
                }
            }
            match &args[1] {
                Spanned { item: v, .. } => {
                    self.value = Some(v.clone());
                }
            }
        }

        Ok(())
    }

    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.edit(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Edit::new());
}
