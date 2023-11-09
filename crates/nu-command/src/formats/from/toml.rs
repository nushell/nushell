use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type,
    Value,
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
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: "'a = 1
b = [1, 2]' | from toml",
                description: "Converts toml formatted string to record",
                result: Some(Value::test_record(record! {
                    "a" =>  Value::test_int(1),
                    "b" =>  Value::test_list(vec![
                        Value::test_int(1),
                        Value::test_int(2)],),
                })),
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

fn convert_toml_to_value(value: &toml::Value, span: Span) -> Value {
    match value {
        toml::Value::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_toml_to_value(x, span))
                .collect();

            Value::list(v, span)
        }
        toml::Value::Boolean(b) => Value::bool(*b, span),
        toml::Value::Float(f) => Value::float(*f, span),
        toml::Value::Integer(i) => Value::int(*i, span),
        toml::Value::Table(k) => Value::record(
            k.iter()
                .map(|(k, v)| (k.clone(), convert_toml_to_value(v, span)))
                .collect(),
            span,
        ),
        toml::Value::String(s) => Value::string(s.clone(), span),
        toml::Value::Datetime(d) => Value::string(d.to_string(), span),
    }
}

pub fn convert_string_to_value(string_input: String, span: Span) -> Result<Value, ShellError> {
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
