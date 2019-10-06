use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    Tagged, TaggedItem, Value,
};

struct Sum {
    total: Option<Tagged<Value>>,
}
impl Sum {
    fn new() -> Sum {
        Sum { total: None }
    }

    fn sum(&mut self, value: Tagged<Value>) -> Result<(), ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Nothing) => Ok(()),
            Value::Primitive(Primitive::Int(i)) => {
                match &self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Int(j)),
                        tag,
                    }) => {
                        //TODO: handle overflow
                        self.total = Some(Value::int(i + j).tagged(*tag));
                        Ok(())
                    }
                    None => {
                        self.total = Some(value.clone());
                        Ok(())
                    }
                    _ => Err(ShellError::labeled_error(
                        "Could not sum non-integer or unrelated types",
                        "source",
                        value.tag,
                    )),
                }
            }
            Value::Primitive(Primitive::Bytes(b)) => {
                match self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Bytes(j)),
                        tag,
                    }) => {
                        //TODO: handle overflow
                        self.total = Some(Value::bytes(b + j).tagged(tag));
                        Ok(())
                    }
                    None => {
                        self.total = Some(value);
                        Ok(())
                    }
                    _ => Err(ShellError::labeled_error(
                        "Could not sum non-integer or unrelated types",
                        "source",
                        value.tag,
                    )),
                }
            }
            x => Err(ShellError::labeled_error(
                format!("Unrecognized type in stream: {:?}", x),
                "source",
                value.tag,
            )),
        }
    }
}

impl Plugin for Sum {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("sum")
            .desc("Sum a column of values.")
            .filter())
    }

    fn begin_filter(&mut self, _: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        self.sum(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        match self.total {
            None => Ok(vec![]),
            Some(ref v) => Ok(vec![ReturnSuccess::value(v.clone())]),
        }
    }
}

fn main() {
    serve_plugin(&mut Sum::new());
}
