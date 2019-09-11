use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, Value,
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

    fn edit(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let value_tag = value.tag();
        match (value.item, self.value.clone()) {
            (obj @ Value::Row(_), Some(v)) => match &self.field {
                Some(f) => match obj.replace_data_at_path(value_tag, &f, v) {
                    Some(v) => return Ok(v),
                    None => {
                        return Err(ShellError::string(
                            "edit could not find place to insert column",
                        ))
                    }
                },
                None => Err(ShellError::string(
                    "edit needs a column when changing a value in a table",
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
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("edit")
            .desc("Edit an existing column to have a new value.")
            .required("Field", SyntaxShape::String)
            .required("Value", SyntaxShape::String)
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Tagged {
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
                Tagged { item: v, .. } => {
                    self.value = Some(v.clone());
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.edit(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Edit::new());
}
