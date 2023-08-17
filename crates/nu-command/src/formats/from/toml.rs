use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SpannedValue,
    Type,
};

#[derive(Clone)]
pub struct FromToml;

impl Command for FromToml {
    fn name(&self) -> &str {
        "from toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("from toml")
            .input_output_types(vec![(Type::String, Type::Record(vec![]))])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Parse text as .toml and create record."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'a = 1' | from toml",
                description: "Converts toml formatted string to record",
                result: Some(SpannedValue::Record {
                    cols: vec!["a".to_string()],
                    vals: vec![SpannedValue::test_int(1)],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "'a = 1
b = [1, 2]' | from toml",
                description: "Converts toml formatted string to record",
                result: Some(SpannedValue::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![
                        SpannedValue::test_int(1),
                        SpannedValue::List {
                            vals: vec![SpannedValue::test_int(1), SpannedValue::test_int(2)],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        __engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let (mut string_input, span, metadata) = input.collect_string_strict(span)?;
        string_input.push('\n');
        Ok(convert_string_to_value(string_input, span)?.into_pipeline_data_with_metadata(metadata))
    }
}

fn convert_toml_to_value(value: &toml::Value, span: Span) -> SpannedValue {
    match value {
        toml::Value::Array(array) => {
            let v: Vec<SpannedValue> = array
                .iter()
                .map(|x| convert_toml_to_value(x, span))
                .collect();

            SpannedValue::List { vals: v, span }
        }
        toml::Value::Boolean(b) => SpannedValue::Bool { val: *b, span },
        toml::Value::Float(f) => SpannedValue::Float { val: *f, span },
        toml::Value::Integer(i) => SpannedValue::Int { val: *i, span },
        toml::Value::Table(k) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for item in k {
                cols.push(item.0.clone());
                vals.push(convert_toml_to_value(item.1, span));
            }

            SpannedValue::Record { cols, vals, span }
        }
        toml::Value::String(s) => SpannedValue::String {
            val: s.clone(),
            span,
        },
        toml::Value::Datetime(d) => SpannedValue::String {
            val: d.to_string(),
            span,
        },
    }
}

pub fn convert_string_to_value(
    string_input: String,
    span: Span,
) -> Result<SpannedValue, ShellError> {
    let result: Result<toml::Value, toml::de::Error> = toml::from_str(&string_input);
    match result {
        Ok(value) => Ok(convert_toml_to_value(&value, span)),

        Err(err) => Err(ShellError::CantConvert {
            to_type: "structured toml data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromToml {})
    }

    #[test]
    fn string_to_toml_value_passes() {
        let input_string = String::from(
            r#"
            command.build = "go build"

            [command.deploy]
            script = "./deploy.sh"
            "#,
        );

        let span = Span::test_data();

        let result = convert_string_to_value(input_string, span);

        assert!(result.is_ok());
    }

    #[test]
    fn string_to_toml_value_fails() {
        let input_string = String::from(
            r#"
            command.build =

            [command.deploy]
            script = "./deploy.sh"
            "#,
        );

        let span = Span::test_data();

        let result = convert_string_to_value(input_string, span);

        assert!(result.is_err());
    }
}
