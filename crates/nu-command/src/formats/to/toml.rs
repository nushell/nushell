use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ToToml;

impl Command for ToToml {
    fn name(&self) -> &str {
        "to toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to toml").category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert table into .toml text"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs an TOML string representing the contents of this table",
            example: r#"[[foo bar]; ["1" "2"]] | to toml"#,
            result: Some(Value::test_string("bar = \"2\"\nfoo = \"1\"\n")),
        }]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        to_toml(input, head)
    }
}

// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
fn helper(v: &Value) -> Result<toml::Value, ShellError> {
    Ok(match &v {
        Value::Bool { val, .. } => toml::Value::Boolean(*val),
        Value::Int { val, .. } => toml::Value::Integer(*val),
        Value::Filesize { val, .. } => toml::Value::Integer(*val),
        Value::Duration { val, .. } => toml::Value::String(val.to_string()),
        Value::Date { val, .. } => toml::Value::String(val.to_string()),
        Value::Range { .. } => toml::Value::String("<Range>".to_string()),
        Value::Float { val, .. } => toml::Value::Float(*val),
        Value::String { val, .. } => toml::Value::String(val.clone()),
        Value::Record { cols, vals, .. } => {
            let mut m = toml::map::Map::new();
            for (k, v) in cols.iter().zip(vals.iter()) {
                m.insert(k.clone(), helper(v)?);
            }
            toml::Value::Table(m)
        }
        Value::List { vals, .. } => toml::Value::Array(toml_list(vals)?),
        Value::Block { .. } => toml::Value::String("<Block>".to_string()),
        Value::Nothing { .. } => toml::Value::String("<Nothing>".to_string()),
        Value::Error { error } => return Err(error.clone()),
        Value::Binary { val, .. } => toml::Value::Array(
            val.iter()
                .map(|x| toml::Value::Integer(*x as i64))
                .collect(),
        ),
        Value::CellPath { val, .. } => toml::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(toml::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(toml::Value::Integer(*val as i64)),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),
        Value::CustomValue { .. } => toml::Value::String("<Custom Value>".to_string()),
    })
}

fn toml_list(input: &[Value]) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(helper(value)?);
    }

    Ok(out)
}

fn toml_into_pipeline_data(
    toml_value: &toml::Value,
    value_type: Type,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match toml::to_string(&toml_value) {
        Ok(serde_toml_string) => Ok(Value::String {
            val: serde_toml_string,
            span,
        }
        .into_pipeline_data()),
        _ => Ok(Value::Error {
            error: ShellError::CantConvert("TOML".into(), value_type.to_string(), span),
        }
        .into_pipeline_data()),
    }
}

fn value_to_toml_value(v: &Value, head: Span) -> Result<toml::Value, ShellError> {
    match v {
        Value::Record { .. } => helper(v),
        Value::List { ref vals, span } => match &vals[..] {
            [Value::Record { .. }, _end @ ..] => helper(v),
            _ => Err(ShellError::UnsupportedInput(
                "Expected a table with TOML-compatible structure from pipeline".to_string(),
                *span,
            )),
        },
        Value::String { val, span } => {
            // Attempt to de-serialize the String
            toml::de::from_str(val).map_err(|_| {
                ShellError::UnsupportedInput(
                    format!("{:?} unable to de-serialize string to TOML", val),
                    *span,
                )
            })
        }
        _ => Err(ShellError::UnsupportedInput(
            format!("{:?} is not a valid top-level TOML", v.get_type()),
            v.span().unwrap_or(head),
        )),
    }
}

fn to_toml(input: PipelineData, span: Span) -> Result<PipelineData, ShellError> {
    let value = input.into_value(span);

    let toml_value = value_to_toml_value(&value, span)?;
    match toml_value {
        toml::Value::Array(ref vec) => match vec[..] {
            [toml::Value::Table(_)] => toml_into_pipeline_data(
                vec.iter().next().expect("this should never trigger"),
                value.get_type(),
                span,
            ),
            _ => toml_into_pipeline_data(&toml_value, value.get_type(), span),
        },
        _ => toml_into_pipeline_data(&toml_value, value.get_type(), span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Spanned;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToToml {})
    }

    #[test]
    fn test_value_to_toml_value() {
        //
        // Positive Tests
        //

        let mut m = indexmap::IndexMap::new();
        m.insert("rust".to_owned(), Value::test_string("editor"));
        m.insert("is".to_owned(), Value::nothing(Span::test_data()));
        m.insert(
            "features".to_owned(),
            Value::List {
                vals: vec![Value::test_string("hello"), Value::test_string("array")],
                span: Span::test_data(),
            },
        );
        let tv = value_to_toml_value(
            &Value::from(Spanned {
                item: m,
                span: Span::test_data(),
            }),
            Span::test_data(),
        )
        .expect("Expected Ok from valid TOML dictionary");
        assert_eq!(
            tv.get("features"),
            Some(&toml::Value::Array(vec![
                toml::Value::String("hello".to_owned()),
                toml::Value::String("array".to_owned())
            ]))
        );
        // TOML string
        let tv = value_to_toml_value(
            &Value::test_string(
                r#"
            title = "TOML Example"

            [owner]
            name = "Tom Preston-Werner"
            dob = 1979-05-27T07:32:00-08:00 # First class dates

            [dependencies]
            rustyline = "4.1.0"
            sysinfo = "0.8.4"
            chrono = { version = "0.4.6", features = ["serde"] }
            "#,
            ),
            Span::test_data(),
        )
        .expect("Expected Ok from valid TOML string");
        assert_eq!(
            tv.get("title").unwrap(),
            &toml::Value::String("TOML Example".to_owned())
        );
        //
        // Negative Tests
        //
        value_to_toml_value(&Value::test_string("not_valid"), Span::test_data())
            .expect_err("Expected non-valid toml (String) to cause error!");
        value_to_toml_value(
            &Value::List {
                vals: vec![Value::test_string("1")],
                span: Span::test_data(),
            },
            Span::test_data(),
        )
        .expect_err("Expected non-valid toml (Table) to cause error!");
    }
}
