use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, Value,
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

    fn add(&self, value: Tagged<Value>) -> Result<Tagged<Value>, ShellError> {
        let value_tag = value.tag();
        match (value.item, self.value.clone()) {
            (obj @ Value::Row(_), Some(v)) => match &self.field {
                Some(f) => match obj.insert_data_at_path(value_tag, &f, v) {
                    Some(v) => return Ok(v),
                    None => {
                        return Err(ShellError::labeled_error(format!(
                            "add could not find place to insert field {:?} {}",
                            obj, f
                        )))
                    }
                },
                None => Err(ShellError::labeled_error(
                    "add needs a column name when adding a value to a table",
                )),
            },
            x => Err(ShellError::labeled_error(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

impl Plugin for Add {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("add")
            .desc("Add a new field to the table.")
            .required("Field", SyntaxShape::String)
            .required("Value", SyntaxShape::String)
            .rest(SyntaxShape::String)
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
                    return Err(ShellError::labeled_error(format!(
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
        Ok(vec![ReturnSuccess::value(self.add(input)?)])
    }
}

fn main() {
    serve_plugin(&mut Add::new());
}
