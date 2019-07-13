use indexmap::IndexMap;
use nu::{
    serve_plugin, Args, CommandConfig, Plugin, PositionalType, Primitive, ReturnSuccess,
    ReturnValue, ShellError, Spanned, SpannedItem, Value,
};

struct Inc {
    inc_by: i64,
}
impl Inc {
    fn new() -> Inc {
        Inc { inc_by: 1 }
    }
}

impl Plugin for Inc {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "inc".to_string(),
            positional: vec![PositionalType::mandatory("Increment")],
            is_filter: true,
            is_sink: false,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }
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

    fn filter(&mut self, input: Spanned<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        let span = input.span;

        match input.item {
            Value::Primitive(Primitive::Int(i)) => Ok(vec![ReturnSuccess::value(
                Value::int(i + self.inc_by).spanned(span),
            )]),
            Value::Primitive(Primitive::Bytes(b)) => Ok(vec![ReturnSuccess::value(
                Value::bytes(b + self.inc_by as u64).spanned(span),
            )]),
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
