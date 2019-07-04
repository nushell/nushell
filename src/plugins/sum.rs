use indexmap::IndexMap;
use nu::{serve_plugin, Args, CommandConfig, Plugin, Primitive, ShellError, Value};

struct Sum;

impl Sum {
    fn new() -> Sum {
        Sum
    }
}

impl Plugin for Sum {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "sum".to_string(),
            mandatory_positional: vec![],
            optional_positional: vec![],
            can_load: vec![],
            can_save: vec![],
            is_filter: false,
            is_sink: true,
            named: IndexMap::new(),
            rest_positional: true,
        })
    }

    fn sink(&mut self, _args: Args, input: Vec<Value>) {
        let mut total = 0i64;

        for v in input {
            match v {
                Value::Primitive(Primitive::Int(i)) => {
                    total += i;
                }
                Value::Primitive(Primitive::Bytes(i)) => {
                    total += i as i64;
                }
                _ => {}
            }
        }

        println!("Result: {}", total);
    }
}

fn main() {
    serve_plugin(&mut Sum::new());
}
