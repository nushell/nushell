use nu_errors::{CoerceInto, ShellError};
use nu_plugin::{serve_plugin, Plugin};
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, UntaggedValue, Value,
};
use nu_source::TaggedItem;

#[derive(Debug)]
struct Average {
    total: Option<Value>,
    count: u64,
}

impl Average {
    fn new() -> Average {
        Average {
            total: None,
            count: 0,
        }
    }

    fn average(&mut self, value: Value) -> Result<(), ShellError> {
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

impl Plugin for Average {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("average")
            .desc("Compute the average of a column of numerical values.")
            .filter())
    }

    fn begin_filter(&mut self, _: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.average(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        match self.total {
            None => Ok(vec![]),
            Some(ref inner) => match &inner.value {
                UntaggedValue::Primitive(Primitive::Int(i)) => {
                    let total: u64 = i
                        .tagged(inner.tag.clone())
                        .coerce_into("converting for average")?;
                    let avg = total as f64 / self.count as f64;
                    let primitive_value: UntaggedValue = Primitive::from(avg).into();
                    let value = primitive_value.into_value(inner.tag.clone());
                    Ok(vec![ReturnSuccess::value(value)])
                }
                UntaggedValue::Primitive(Primitive::Bytes(bytes)) => {
                    let avg = *bytes as f64 / self.count as f64;
                    let primitive_value: UntaggedValue = Primitive::from(avg).into();
                    let tagged_value = primitive_value.into_value(inner.tag.clone());
                    Ok(vec![ReturnSuccess::value(tagged_value)])
                }
                _ => Ok(vec![]),
            },
        }
    }
}

fn main() {
    serve_plugin(&mut Average::new());
}
