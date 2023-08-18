use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{PluginExample, ShellError, Span, Value};

pub const CMD_NAME: &str = "from ini";

pub fn from_ini_call(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let span = input.span();
    let input_string = input.as_string()?;
    let head = call.head;

    let ini_config: Result<ini::Ini, ini::ParseError> = ini::Ini::load_from_str(&input_string);
    match ini_config {
        Ok(config) => {
            let mut sections: Vec<String> = Vec::new();
            let mut sections_key_value_pairs: Vec<Value> = Vec::new();

            for (section, properties) in config.iter() {
                let mut keys_for_section: Vec<String> = Vec::new();
                let mut values_for_section: Vec<Value> = Vec::new();

                // section
                match section {
                    Some(section_name) => {
                        sections.push(section_name.to_owned());
                    }
                    None => {
                        // Section (None) allows for key value pairs without a section
                        if !properties.is_empty() {
                            sections.push(String::new());
                        }
                    }
                }

                // section's key value pairs
                for (key, value) in properties.iter() {
                    keys_for_section.push(key.to_owned());
                    values_for_section.push(Value::String {
                        val: value.to_owned(),
                        span,
                    });
                }

                // section with its key value pairs
                // Only add section if contains key,value pair
                if !properties.is_empty() {
                    sections_key_value_pairs.push(Value::Record {
                        cols: keys_for_section,
                        vals: values_for_section,
                        span,
                    });
                }
            }

            // all sections with all its key value pairs
            Ok(Value::Record {
                cols: sections,
                vals: sections_key_value_pairs,
                span,
            })
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
        result: Some(Value::Record {
            cols: vec!["foo".to_string()],
            vals: vec![Value::Record {
                cols: vec!["a".to_string(), "b".to_string()],
                vals: vec![Value::test_string("1"), Value::test_string("2")],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        }),
    }]
}
