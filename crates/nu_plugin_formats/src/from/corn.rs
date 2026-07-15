use crate::FormatCmdsPlugin;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, ShellError, Signature, Span, Type, Value, record,
};

#[derive(Clone)]
pub struct FromCorn;

impl SimplePluginCommand for FromCorn {
    type Plugin = FormatCmdsPlugin;
    fn name(&self) -> &str {
        "from corn"
    }

    fn description(&self) -> &str {
        "Convert CORN text into structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from corn")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "'{ a = 1 }' | from corn",
                description: "Converts corn formatted string to table.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: "'{ a = 1 b = [1 2] }' | from corn",
                description: "Converts corn formatted string to table.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                })),
            },
        ]
    }

    fn run(
        &self,
        _plugin: &FormatCmdsPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let span = call.head;

        let string_input = input.coerce_str()?;

        if string_input.is_empty() {
            return Ok(Value::nothing(span));
        }

        Ok(try_str_to_value(&string_input, span)?)
    }
}

pub fn try_str_to_value(input: &str, span: Span) -> Result<Value, LabeledError> {
    let result = corn::parse(input);
    match result {
        Ok(value) => Ok(convert_corn_to_value(&value, span)),

        Err(err) => Err(ShellError::CantConvert {
            to_type: "structured corn data".into(),
            from_type: "string".into(),
            span,
            help: Some(err.to_string()),
        }
        .into()),
    }
}

fn convert_corn_to_value(value: &corn::Value<'_>, span: Span) -> Value {
    match value {
        corn::Value::Object(k) => Value::record(
            k.iter()
                .map(|(k, v)| (k.clone().to_string(), convert_corn_to_value(v, span)))
                .collect(),
            span,
        ),
        corn::Value::Array(array) => {
            let v: Vec<Value> = array
                .iter()
                .map(|x| convert_corn_to_value(x, span))
                .collect();

            Value::list(v, span)
        }
        corn::Value::Boolean(b) => Value::bool(*b, span),
        corn::Value::Float(f) => Value::float(*f, span),
        corn::Value::Integer(i) => Value::int(*i, span),
        corn::Value::String(s) => Value::string(s.clone(), span),
        corn::Value::Null(_) => Value::nothing(span),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromCorn)
    }

    #[test]
    fn corn_parse_success_not_affected() {
        let input = "{a = 1 b = [2 3]}";
        let result = try_str_to_value(input, Span::test_data());
        assert!(result.is_ok(), "valid CORN should still parse");
    }
}
