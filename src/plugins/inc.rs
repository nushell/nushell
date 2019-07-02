use nu::{serve_plugin, Args, Plugin, Primitive, ReturnValue, ShellError, Spanned, Value};

struct Inc {
    inc_by: i64,
}
impl Inc {
    fn new() -> Inc {
        Inc { inc_by: 1 }
    }
}

impl Plugin for Inc {
    fn begin_filter(&mut self, args: Args) -> Result<(), ShellError> {
        if let Some(args) = args.positional {
            for arg in args {
                match arg {
                    Spanned {
                        item: Value::Primitive(Primitive::Int(i)),
                        ..
                    } => {
                        self.inc_by = i;
                    }
                    _ => return Err(ShellError::string("Unrecognized type in params")),
                }
            }
        }

        Ok(())
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match input {
            Value::Primitive(Primitive::Int(i)) => {
                Ok(vec![ReturnValue::Value(Value::int(i + self.inc_by))])
            }
            Value::Primitive(Primitive::Bytes(b)) => Ok(vec![ReturnValue::Value(Value::bytes(
                b + self.inc_by as u64,
            ))]),
            x => Err(ShellError::string(format!(
                "Unrecognized type in stream: {:?}",
                x
            ))),
        }
    }
}

fn main() {
    serve_plugin(&mut Inc::new());
}
