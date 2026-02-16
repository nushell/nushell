use crate::FormatCmdsPlugin;

use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, Record, ShellError, Signature, Type, Value, record,
};

pub struct FromIni;

impl SimplePluginCommand for FromIni {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "from ini"
    }

    fn description(&self) -> &str {
        "Parse text as .ini and create table."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::String, Type::record())])
            .switch("no-quote", "Disable quote handling for values", None)
            .switch(
                "no-escape",
                "Disable escape sequence handling for values",
                None,
            )
            .switch(
                "indented-multiline-value",
                "Allow values to continue on indented lines",
                None,
            )
            .switch(
                "preserve-key-leading-whitespace",
                "Preserve leading whitespace in keys",
                None,
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        examples()
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = input.span();
        let input_string = input.coerce_str()?;
        let head = call.head;
        let mut parse_option = ini::ParseOption::default();

        if call.has_flag("no-quote")? {
            parse_option.enabled_quote = false;
        }

        if call.has_flag("no-escape")? {
            parse_option.enabled_escape = false;
        }

        if call.has_flag("indented-multiline-value")? {
            parse_option.enabled_indented_mutiline_value = true;
        }

        if call.has_flag("preserve-key-leading-whitespace")? {
            parse_option.enabled_preserve_key_leading_whitespace = true;
        }

        let ini_config: Result<ini::Ini, ini::ParseError> =
            ini::Ini::load_from_str_opt(&input_string, parse_option);
        match ini_config {
            Ok(config) => {
                let mut sections = Record::new();

                for (section, properties) in config.iter() {
                    let mut section_record = Record::new();

                    // section's key value pairs
                    for (key, value) in properties.iter() {
                        section_record.push(key, Value::string(value, span));
                    }

                    let section_record = Value::record(section_record, span);

                    // section
                    match section {
                        Some(section_name) => {
                            sections.push(section_name, section_record);
                        }
                        None => {
                            // Section (None) allows for key value pairs without a section
                            if !properties.is_empty() {
                                sections.push(String::new(), section_record);
                            }
                        }
                    }
                }

                // all sections with all its key value pairs
                Ok(Value::record(sections, span))
            }
            Err(err) => Err(ShellError::UnsupportedInput {
                msg: format!("Could not load ini: {err}"),
                input: "value originates from here".into(),
                msg_span: head,
                input_span: span,
            }
            .into()),
        }
    }
}

pub fn examples() -> Vec<Example<'static>> {
    vec![
        Example {
            example: "'[foo]
a=1
b=2' | from ini",
            description: "Converts ini formatted string to record",
            result: Some(Value::test_record(record! {
                "foo" => Value::test_record(record! {
                    "a" =>  Value::test_string("1"),
                    "b" =>  Value::test_string("2"),
                }),
            })),
        },
        Example {
            example: "'[start]
file=C:\\Windows\\System32\\xcopy.exe' | from ini --no-escape",
            description: "Disable escaping to keep backslashes literal",
            result: Some(Value::test_record(record! {
                "start" => Value::test_record(record! {
                    "file" => Value::test_string(r"C:\Windows\System32\xcopy.exe"),
                }),
            })),
        },
    ]
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("formats", crate::FormatCmdsPlugin.into())?.test_command_examples(&FromIni)
}
