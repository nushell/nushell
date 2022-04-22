use gjson::Value as gjValue;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "query json"
    }

    fn signature(&self) -> Signature {
        Signature::build("query json")
            .required(
                "query",
                SyntaxShape::String,
                "A GJSON path to execute against the input JSON",
            )
            .category(Category::Query)
    }

    fn usage(&self) -> &str {
        "Execute a GJSON query against a JSON string"
    }

    fn extra_usage(&self) -> &str {
        "GJSON syntax: https://github.com/tidwall/gjson/blob/master/SYNTAX.md"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query_string: Spanned<String> = call.req(engine_state, stack, 0)?;
        let input_value = input.into_value(call.head);
        let input_span = input_value.span()?;

        if let Value::String { val: json, span } = input_value {
            // Validate the json before trying to query it
            if !gjson::valid(&json) {
                return Err(ShellError::UnsupportedInput(
                    "Input is not valid JSON".into(),
                    span,
                ));
            }

            let val: gjValue = gjson::get(&json, &query_string.item);

            if query_contains_modifiers(&query_string.item) {
                let json_str = val.json();
                return Ok(Value::string(json_str, call.head).into_pipeline_data());
            } else {
                return Ok(convert_gjson_value_to_nu_value(&val, &call.head).into_pipeline_data());
            }
        }

        Err(ShellError::PipelineMismatch(
            "string input".into(),
            call.head,
            input_span,
        ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get a value from a JSON string",
                example: r#"`{"foo": 1, "bar": 2}` | query json "bar""#,
                result: Some(Value::Int {
                    val: 2,
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Query a JSON file",
                example: r#"open results.json --raw | query json "tests.0.name""#,
                result: None,
            },
        ]
    }
}

fn query_contains_modifiers(query: &str) -> bool {
    // https://github.com/tidwall/gjson.rs documents 7 modifiers as of 4/19/21
    // Some of these modifiers mean we really need to output the data as a string
    // instead of tabular data. Others don't matter.

    // Output as String
    // @ugly: Remove all whitespace from a json document.
    // @pretty: Make the json document more human readable.
    query.contains("@ugly") || query.contains("@pretty")

    // Output as Tablular
    // Since it's output as tabular, which is our default, we can just ignore these
    // @reverse: Reverse an array or the members of an object.
    // @this: Returns the current element. It can be used to retrieve the root element.
    // @valid: Ensure the json document is valid.
    // @flatten: Flattens an array.
    // @join: Joins multiple objects into a single object.
}

fn convert_gjson_value_to_nu_value(v: &gjValue, span: &Span) -> Value {
    match v.kind() {
        gjson::Kind::Array => {
            let mut vals = vec![];
            v.each(|_k, v| {
                vals.push(convert_gjson_value_to_nu_value(&v, span));
                true
            });

            Value::List { vals, span: *span }
        }
        gjson::Kind::Null => Value::nothing(*span),
        gjson::Kind::False => Value::boolean(false, *span),
        gjson::Kind::Number => {
            let str_value = v.str();
            if str_value.contains('.') {
                Value::float(v.f64(), *span)
            } else {
                Value::int(v.i64(), *span)
            }
        }
        gjson::Kind::String => Value::string(v.str(), *span),
        gjson::Kind::True => Value::boolean(true, *span),
        gjson::Kind::Object => {
            let mut cols = vec![];
            let mut vals = vec![];
            v.each(|k, v| {
                cols.push(k.to_string());
                vals.push(convert_gjson_value_to_nu_value(&v, span));
                true
            });
            Value::Record {
                cols,
                vals,
                span: *span,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(SubCommand {})
    }
}
