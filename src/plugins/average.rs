use nu::{
    serve_plugin, CallInfo, CoerceInto, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError,
    Signature, Tagged, TaggedItem, Value,
};

#[derive(Debug)]
struct Average {
    total: Option<Tagged<Value>>,
    count: u64,
}

impl Average {
    fn new() -> Average {
        Average {
            total: None,
            count: 0,
        }
    }

    fn average(&mut self, value: Tagged<Value>) -> Result<(), ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Nothing) => Ok(()),
            Value::Primitive(Primitive::Int(i)) => match &self.total {
                Some(Tagged {
                    item: Value::Primitive(Primitive::Int(j)),
                    tag,
                }) => {
                    self.total = Some(Value::int(i + j).tagged(tag));
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
            Value::Primitive(Primitive::Bytes(b)) => match &self.total {
                Some(Tagged {
                    item: Value::Primitive(Primitive::Bytes(j)),
                    tag,
                }) => {
                    self.total = Some(Value::bytes(b + j).tagged(tag));
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

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        self.average(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        match self.total {
            None => Ok(vec![]),
            Some(ref inner) => {
                match inner.item() {
                    Value::Primitive(Primitive::Int(i)) => {
                        let total: u64 = i
                            .tagged(inner.tag.clone())
                            .coerce_into("converting for average")?;
                        let avg = total as f64 / self.count as f64;
                        let primitive_value: Value = Primitive::from(avg).into();
                        let tagged_value = primitive_value.tagged(inner.tag.clone());
                        Ok(vec![ReturnSuccess::value(tagged_value)])
                    }
                    Value::Primitive(Primitive::Bytes(bytes)) => {
                        // let total: u64 = b.tagged(inner.tag.clone()).coerce_into("converting for average")?;
                        let avg = *bytes as f64 / self.count as f64;
                        let primitive_value: Value = Primitive::from(avg).into();
                        let tagged_value = primitive_value.tagged(inner.tag.clone());
                        Ok(vec![ReturnSuccess::value(tagged_value)])
                    }
                    _ => Ok(vec![]),
                }
            }
        }
    }
}

fn main() {
    serve_plugin(&mut Average::new());
}
