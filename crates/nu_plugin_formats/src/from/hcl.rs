use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, ShellError, Span, Value};
use serde_json::Value as SerdeJsonValue;

pub const CMD_NAME: &str = "from hcl";

pub fn from_hcl_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let input_string = input.as_string()?;

    let hcl = hcl::from_str(&input_string).map_err(|_| ShellError::CantConvert {
        to_type: "structured hcl data".into(),
        from_type: "string".into(),
        span: call.head,
        help: None,
    })?;

    Ok(convert_sjson_to_value(&hcl, call.head))
}
//shamelessly stolen from fdncred
pub fn convert_sjson_to_value(value: &SerdeJsonValue, span: Span) -> Value {
    match value {
        SerdeJsonValue::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_sjson_to_value(x, span))
                .collect();

            Value::List { vals: v, span }
        }
        SerdeJsonValue::Bool(b) => Value::Bool { val: *b, span },
        SerdeJsonValue::Number(f) => {
            if f.is_f64() {
                Value::Float {
                    val: f.as_f64().expect("Failed to convert to f64"),
                    span,
                }
            } else {
                Value::Int {
                    val: f.as_i64().expect("Failed to convert to i64"),
                    span,
                }
            }
        }
        SerdeJsonValue::Null => Value::Nothing { span },
        SerdeJsonValue::Object(k) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for item in k {
                cols.push(item.0.clone());
                vals.push(convert_sjson_to_value(item.1, span));
            }

            Value::Record { cols, vals, span }
        }
        SerdeJsonValue::String(s) => Value::String {
            val: s.clone(),
            span,
        },
    }
}

pub fn examples() -> Vec<PluginExample> {
    vec![PluginExample {
        description: "Convert .hcl data into record".into(),
        example: "'provider \"aws\" {
  region = \"us-east-1\"
}
resource \"aws_instance\" \"web\" {
  ami           = \"ami-a1b2c3d4\"
  instance_type = \"t2.micro\"
}' | from hcl"
            .into(),
        result: Some(Value::Record {
            cols: vec!["provider".to_string(), "resource".to_string()],
            vals: vec![
                Value::Record {
                    cols: vec!["aws".to_string()],
                    vals: vec![Value::Record {
                        cols: vec!["region".to_string()],
                        vals: vec![Value::test_string("us-east-1")],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                },
                Value::Record {
                    cols: vec!["aws_instance".to_string()],
                    vals: vec![Value::Record {
                        cols: vec!["web".to_string()],
                        vals: vec![Value::Record {
                            cols: vec!["ami".to_string(), "instance_type".to_string()],
                            vals: vec![
                                Value::test_string("ami-a1b2c3d4"),
                                Value::test_string("t2.micro"),
                            ],
                            span: Span::test_data(),
                        }],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                },
            ],
            span: Span::test_data(),
        }),
    }]
}
