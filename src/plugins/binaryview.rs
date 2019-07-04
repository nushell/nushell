use indexmap::IndexMap;
use nu::{serve_plugin, Args, CommandConfig, Plugin, Primitive, ShellError, Value};

struct BinaryView;

impl BinaryView {
    fn new() -> BinaryView {
        BinaryView
    }
}

impl Plugin for BinaryView {
    fn config(&mut self) -> Result<CommandConfig, ShellError> {
        Ok(CommandConfig {
            name: "binaryview".to_string(),
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
        for v in input {
            match v {
                Value::Binary(b) => {
                    view_binary(&b);
                }
                _ => {}
            }
        }
    }
}

fn view_binary(b: &[u8]) {
    use pretty_hex::*;
    println!("{:?}", b.hex_dump());
}

fn main() {
    serve_plugin(&mut BinaryView::new());
}
