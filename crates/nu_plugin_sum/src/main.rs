use nu_errors::ShellError;
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value,
};

struct Sum {
    total: Option<Value>,
}
impl Sum {
    fn new() -> Sum {
        Sum { total: None }
    }

    fn sum(&mut self, value: Value) -> Result<(), ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Nothing) => Ok(()),
            UntaggedValue::Primitive(Primitive::Int(i)) => {
                match &self.total {
                    Some(Value {
                        value: UntaggedValue::Primitive(Primitive::Int(j)),
                        tag,
                    }) => {
                        //TODO: handle overflow
                        self.total = Some(UntaggedValue::int(i + j).into_value(tag));
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
            UntaggedValue::Primitive(Primitive::Bytes(b)) => {
                match &self.total {
                    Some(Value {
                        value: UntaggedValue::Primitive(Primitive::Bytes(j)),
                        tag,
                    }) => {
                        //TODO: handle overflow
                        self.total = Some(UntaggedValue::bytes(b + j).into_value(tag));
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

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
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
