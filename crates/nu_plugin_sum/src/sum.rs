use nu_errors::ShellError;
use nu_protocol::{Primitive, UntaggedValue, Value};

#[derive(Default)]
pub struct Sum {
    pub total: Option<Value>,
}

impl Sum {
    pub fn new() -> Sum {
        Sum { total: None }
    }

    pub fn sum(&mut self, value: Value) -> Result<(), ShellError> {
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
