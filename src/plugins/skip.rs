use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Signature, Plugin, Primitive, ReturnSuccess, ReturnValue,
    ShellError, Spanned, Value,
};

struct NewSkip {
    skip_amount: i64,
}
impl NewSkip {
    fn new() -> NewSkip {
        NewSkip { skip_amount: 0 }
    }
}

impl Plugin for NewSkip {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature {
            name: "skip".to_string(),
            positional: vec![],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<(), ShellError> {
        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    Spanned {
                        item: Value::Primitive(Primitive::Int(i)),
                        ..
                    } => {
                        self.skip_amount = i;
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in params",
                            "expected an integer",
                            arg.span,
                        ))
                    }
                }
            }
        }

        Ok(())
    }

    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        if self.skip_amount == 0 {
            Ok(vec![ReturnSuccess::value(input)])
        } else {
            self.skip_amount -= 1;
            Ok(vec![])
        }
    }
}

fn main() {
    serve_plugin(&mut NewSkip::new());
}
