use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

extern crate ini as RustIni;

#[derive(Clone)]
pub struct FromIni;

impl Command for FromIni {
    fn name(&self) -> &str {
        "from ini"
    }

    fn signature(&self) -> Signature {
        Signature::build("from ini")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .ini and create record"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "'[foo]
a=1
b=2' | from ini",
            description: "Converts ini formatted string to record",
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

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        from_ini(input, head)
    }
}

pub fn from_ini_string_to_value(
    s: String,
    span: Span,
    val_span: Span,
) -> Result<Value, ShellError> {
    let ini: Result<RustIni::Ini, RustIni::ParseError> = RustIni::Ini::load_from_str(&s);

    match ini {
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
                        sections.push(String::new());
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
                sections_key_value_pairs.push(Value::Record {
                    cols: keys_for_section,
                    vals: values_for_section,
                    span,
                });
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
            span,
            val_span,
        )),
    }
}

fn from_ini(input: PipelineData, head: Span) -> Result<PipelineData, ShellError> {
    let (concat_string, span, metadata) = input.collect_string_strict(head)?;

    match from_ini_string_to_value(concat_string, head, span) {
        Ok(x) => Ok(x.into_pipeline_data_with_metadata(metadata)),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromIni {})
    }

    #[test]
    fn read_ini_config_passes() {
        let ini_test_config = r"
        min-width=450
        max-width=820

        [normal]
        sound-file=/usr/share/sounds/freedesktop/stereo/dialog-information.oga

        [critical]
        border-color=FAB387ff
        default-timeout=20
        sound-file=/usr/share/sounds/freedesktop/stereo/dialog-warning.oga
        ";

        let result = from_ini_string_to_value(
            ini_test_config.to_owned(),
            Span::test_data(),
            Span::test_data(),
        );

        assert!(result.is_ok());
    }
}
