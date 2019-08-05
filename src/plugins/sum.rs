use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, CommandConfig, Plugin, Primitive, ReturnSuccess, ReturnValue,
    ShellError, Tag, Tagged, Value,
};

struct Sum {
    total: Option<Tagged<Value>>,
}
impl Sum {
    fn new() -> Sum {
        Sum { total: None }
    }

    fn sum(&mut self, value: Tagged<Value>) -> Result<(), ShellError> {
        match value.item {
            Value::Primitive(Primitive::Int(i)) => {
                match self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Int(j)),
                        tag: Tag { span, .. },
                    }) => {
                        //TODO: handle overflow
                        self.total =
                            Some(Tagged::from_simple_spanned_item(Value::int(i + j), span));
                        Ok(())
                    }
                    None => {
                        self.total = Some(value);
                        Ok(())
                    }
                    _ => Err(ShellError::string(format!(
                        "Could not sum non-integer or unrelated types"
                    ))),
                }
            }
            Value::Primitive(Primitive::Bytes(b)) => {
                match self.total {
                    Some(Tagged {
                        item: Value::Primitive(Primitive::Bytes(j)),
                        tag: Tag { span, .. },
                    }) => {
                        //TODO: handle overflow
                        self.total =
                            Some(Tagged::from_simple_spanned_item(Value::bytes(b + j), span));
                        Ok(())
                    }
                    None => {
                        self.total = Some(value);
                        Ok(())
                    }
                    _ => Err(ShellError::string(format!(
                        "Could not sum non-integer or unrelated types"
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

impl Plugin for Sum {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "sum".to_string(),
            positional: vec![],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
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
