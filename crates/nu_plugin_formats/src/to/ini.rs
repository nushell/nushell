use crate::FormatCmdsPlugin;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand, SimplePluginCommand};
use nu_protocol::{Category, Example, LabeledError, Signature, Type, Value};

pub(crate) struct ToIni;

impl SimplePluginCommand for ToIni {
    type Plugin = FormatCmdsPlugin;

    fn name(&self) -> &str {
        "to ini"
    }

    fn description(&self) -> &str {
        "Convert a record into .ini text."
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .input_output_types(vec![(Type::record(), Type::String)])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "{ foo: { a: '1', b: '2' } } | to ini",
                description: "Convert a record of sections to ini text",
                result: Some(Value::test_string("[foo]\na=1\nb=2\n")),
            },
            Example {
                example: "{ port: 8080, debug: true, server: { host: localhost } } | to ini",
                description: "Top-level values that are not records become global properties, written above the first section",
                result: Some(Value::test_string(
                    "port=8080\ndebug=true\n\n[server]\nhost=localhost\n",
                )),
            },
            Example {
                example: "'[foo]
a=1' | from ini | to ini",
                description: "Ini text survives a round trip through from ini",
                result: Some(Value::test_string("[foo]\na=1\n")),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        _call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let Value::Record { val: record, .. } = input else {
            return Err(LabeledError::new("Cannot convert to ini")
                .with_label("expected a record", input.span()));
        };

        let mut config = ini::Ini::new();

        // Ini::new pre-seeds the general section at slot 0, so global keys always
        // land above the first [section] no matter when we insert them.
        for (key, value) in record.iter() {
            match value {
                Value::Record { val: section, .. } => {
                    let section_name = (!key.is_empty()).then(|| key.to_owned());
                    let props = config.entry(section_name).or_insert_with(Default::default);
                    for (prop_key, prop_value) in section.iter() {
                        props.insert(prop_key, property_text(prop_key, prop_value)?);
                    }
                }
                _ => {
                    config
                        .general_section_mut()
                        .insert(key, property_text(key, value)?);
                }
            }
        }

        let write_option = ini::WriteOption {
            line_separator: ini::LineSeparator::CR,
            ..Default::default()
        };
        let mut out = Vec::new();
        config
            .write_to_opt(&mut out, write_option)
            .and_then(|_| String::from_utf8(out).map_err(std::io::Error::other))
            .map(|text| Value::string(text, input.span()))
            .map_err(|err| {
                LabeledError::new("Cannot convert to ini").with_label(err.to_string(), input.span())
            })
    }
}

fn property_text(key: &str, value: &Value) -> Result<String, LabeledError> {
    match value {
        Value::Nothing { .. } => Ok(String::new()),
        Value::Record { .. } => Err(LabeledError::new("Cannot convert to ini").with_label(
            format!("`{key}` is a nested record, and ini sections cannot nest"),
            value.span(),
        )),
        Value::List { .. } => Err(LabeledError::new("Cannot convert to ini").with_label(
            format!("`{key}` is a list, and ini has no list values"),
            value.span(),
        )),
        other => other.coerce_string().map_err(Into::into),
    }
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("formats", crate::FormatCmdsPlugin.into())?.test_command_examples(&ToIni)
}
