use nu::{
    serve_plugin, CoerceInto, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    Tagged, TaggedItem, Value,
};

#[derive(Debug)]
struct Average {
    total: Option<Tagged<Value>>,
    count: u64,
}

impl Average {
    fn new() -> Average {
        Average { total: None, count: 1 }
    }

    fn average(&mut self, value: Tagged<Value>) -> Result<(), ShellError> {
        match value.item() {
            Value::Primitive(Primitive::Nothing) => Ok(()),
            Value::Primitive(Primitive::Int(i)) => {
                match &self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Int(j)),
                        tag,
                    }) => {
                        self.total = Some(Value::int(i + j).tagged(tag));
                        self.count = self.count + 1;
                        Ok(())
                    }
                    None => {
                        self.total = Some(value.clone());
                        Ok(())
                    }
                    _ => Err(ShellError::string(format!(
                        "Could not calculate average of non-integer or unrelated types"
                    ))),
                }
            }
            Value::Primitive(Primitive::Bytes(b)) => {
                match self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Bytes(j)),
                        tag,
                    }) => {
                        self.total = Some(Value::int(b + j).tagged(tag));
                        self.count = self.count + 1;
                        Ok(())
                    }
                    None => {
                        self.total = Some(value);
                        Ok(())
                    }
                    _ => Err(ShellError::string(format!(
                        "Could not calculate average of non-integer or unrelated types"
                    ))),
                }
            }
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
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
            Some(ref v) =>  {
                match v.item() {
                    Value::Primitive(Primitive::Int(i)) => {
                        let total: u64 = i.tagged(v.tag).coerce_into("converting for average")?;
                        let avg = total as f64 / self.count as f64;
                        let decimal_value: Value=  Primitive::from(avg).into();
                        let tagged_value = decimal_value.tagged(v.tag);
                        Ok(vec![ReturnSuccess::value(tagged_value)])
                    }
                    _ => unreachable!()

                }
            },
        }
    }
}

fn main() {
    serve_plugin(&mut Average::new());
}

