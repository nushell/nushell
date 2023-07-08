use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, Record, ShellError, Value};

pub const CMD_NAME: &str = "from ini";

pub fn from_ini_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let span = input.span().unwrap_or(call.head);
    let input_string = input.as_string()?;
    let head = call.head;

    let ini_config: Result<ini::Ini, ini::ParseError> = ini::Ini::load_from_str(&input_string);
    match ini_config {
        Ok(config) => {
            let record = config
                .iter()
                .map(|(section, properties)| {
                    let section_name = section.unwrap_or_default().to_owned();

                    // section's key value pairs
                    let properties = properties
                        .iter()
                        .map(|(key, value)| (key.to_owned(), Value::string(value.to_owned(), span)))
                        .collect();

                    // section with its key value pairs
                    (section_name, Value::record(properties, span))
                })
                .collect();

            Ok(Value::record(record, span))
        }
        Err(err) => Err(ShellError::UnsupportedInput(
            format!("Could not load ini: {err}"),
            "value originates from here".into(),
            head,
            span,
        )
        .into()),
    }
}

pub fn examples() -> Vec<PluginExample> {
    vec![PluginExample {
        example: "'[foo]
a=1
b=2' | from ini"
            .into(),
        description: "Converts ini formatted string to record".into(),
        result: Some(Value::test_record(Record {
            cols: vec!["foo".to_string()],
            vals: vec![Value::test_record(Record {
                cols: vec!["a".to_string(), "b".to_string()],
                vals: vec![Value::test_string("1"), Value::test_string("2")],
            })],
        })),
    }]
}
