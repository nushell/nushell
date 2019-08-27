use indexmap::IndexMap;
use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxType, Tagged, Value,
};

struct Skip {
    skip_amount: i64,
}
impl Skip {
    fn new() -> Skip {
        Skip { skip_amount: 0 }
    }
}

impl Plugin for Skip {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature {
            name: "skip".to_string(),
            positional: vec![],
            is_filter: true,
            named: IndexMap::new(),
            rest_positional: Some(SyntaxType::Number),
        })
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    Tagged {
                        item: Value::Primitive(Primitive::Int(i)),
                        ..
                    } => {
                        self.skip_amount = i;
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in params",
                            "expected an integer",
                            arg.span(),
                        ))
                    }
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        if self.skip_amount == 0 {
            Ok(vec![ReturnSuccess::value(input)])
        } else {
            self.skip_amount -= 1;
            Ok(vec![])
        }
    }
}

fn main() {
    serve_plugin(&mut Skip::new());
}
