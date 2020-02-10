use nu_errors::ShellError;
use nu_protocol::{Primitive, UntaggedValue, Value};

#[derive(Debug, Default)]
pub struct Average {
    pub total: Option<Value>,
    pub count: u64,
}

impl Average {
    pub fn new() -> Average {
        Average {
            total: None,
            count: 0,
        }
    }

    pub fn average(&mut self, value: Value) -> Result<(), ShellError> {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Nothing) => Ok(()),
            UntaggedValue::Primitive(Primitive::Int(i)) => match &self.total {
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::Int(j)),
                    tag,
                }) => {
                    self.total = Some(UntaggedValue::int(i + j).into_value(tag));
                    self.count += 1;
                    Ok(())
                }
                None => {
                    self.total = Some(value.clone());
                    self.count += 1;
                    Ok(())
                }
                _ => Err(ShellError::labeled_error(
                    "Could calculate average of non-integer or unrelated types",
                    "source",
                    value.tag,
                )),
            },
            UntaggedValue::Primitive(Primitive::Bytes(b)) => match &self.total {
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::Bytes(j)),
                    tag,
                }) => {
                    self.total = Some(UntaggedValue::bytes(b + j).into_value(tag));
                    self.count += 1;
                    Ok(())
                }
                None => {
                    self.total = Some(value);
                    self.count += 1;
                    Ok(())
                }
                _ => Err(ShellError::labeled_error(
                    "Could calculate average of non-integer or unrelated types",
                    "source",
                    value.tag,
                )),
            },
            x => Err(ShellError::labeled_error(
                format!("Unrecognized type in stream: {:?}", x),
                "source",
                value.tag,
            )),
        }
    }
}
