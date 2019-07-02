use nu::{serve_plugin, Args, Plugin, Primitive, Value};

struct Sum;

impl Sum {
    fn new() -> Sum {
        Sum
    }
}

impl Plugin for Sum {
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
