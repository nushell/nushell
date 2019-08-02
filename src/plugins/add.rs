use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Signature, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Spanned, Value,
};

struct Add {
    field: Option<String>,
    value: Option<Value>,
}
impl Add {
    fn new() -> Add {
        Add {
            field: None,
            value: None,
        }
    }

    fn add(&self, value: Spanned<Value>) -> Result<Spanned<Value>, ShellError> {
        match (value.item, self.value.clone()) {
            (obj @ Value::Object(_), Some(v)) => match &self.field {
                Some(f) => match obj.insert_data_at_path(value.span, &f, v) {
                    Some(v) => return Ok(v),
                    None => {
                        return Err(ShellError::string(
                            "add could not find place to insert field",
                        ))
                    }
                },
                None => Err(ShellError::string(
                    "add needs a field when adding a value to an object",
                )),
            },
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Add {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature {
            name: "add".to_string(),
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
        Ok(vec![ReturnSuccess::value(self.add(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Add::new());
}
