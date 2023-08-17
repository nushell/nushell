use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SpannedValue,
    Type,
};

#[derive(Clone)]
pub struct ToToml;

impl Command for ToToml {
    fn name(&self) -> &str {
        "to toml"
    }

    fn signature(&self) -> Signature {
        Signature::build("to toml")
            .input_output_types(vec![(Type::Record(vec![]), Type::String)])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Convert record into .toml text."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Outputs an TOML string representing the contents of this record",
            example: r#"{foo: 1 bar: 'qwe'} | to toml"#,
            result: Some(SpannedValue::test_string("bar = \"qwe\"\nfoo = 1\n")),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        to_toml(engine_state, input, head)
    }
}

// Helper method to recursively convert nu_protocol::Value -> toml::Value
// This shouldn't be called at the top-level
fn helper(engine_state: &EngineState, v: &SpannedValue) -> Result<toml::Value, ShellError> {
    Ok(match &v {
        SpannedValue::Bool { val, .. } => toml::Value::Boolean(*val),
        SpannedValue::Int { val, .. } => toml::Value::Integer(*val),
        SpannedValue::Filesize { val, .. } => toml::Value::Integer(*val),
        SpannedValue::Duration { val, .. } => toml::Value::String(val.to_string()),
        SpannedValue::Date { val, .. } => toml::Value::String(val.to_string()),
        SpannedValue::Range { .. } => toml::Value::String("<Range>".to_string()),
        SpannedValue::Float { val, .. } => toml::Value::Float(*val),
        SpannedValue::String { val, .. } => toml::Value::String(val.clone()),
        SpannedValue::Record { cols, vals, .. } => {
            let mut m = toml::map::Map::new();
            for (k, v) in cols.iter().zip(vals.iter()) {
                m.insert(k.clone(), helper(engine_state, v)?);
            }
            toml::Value::Table(m)
        }
        SpannedValue::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            helper(engine_state, &collected)?
        }
        SpannedValue::List { vals, .. } => toml::Value::Array(toml_list(engine_state, vals)?),
        SpannedValue::Block { span, .. } => {
            let code = engine_state.get_span_contents(*span);
            let code = String::from_utf8_lossy(code).to_string();
            toml::Value::String(code)
        }
        SpannedValue::Closure { span, .. } => {
            let code = engine_state.get_span_contents(*span);
            let code = String::from_utf8_lossy(code).to_string();
            toml::Value::String(code)
        }
        SpannedValue::Nothing { .. } => toml::Value::String("<Nothing>".to_string()),
        SpannedValue::Error { error } => return Err(*error.clone()),
        SpannedValue::Binary { val, .. } => toml::Value::Array(
            val.iter()
                .map(|x| toml::Value::Integer(*x as i64))
                .collect(),
        ),
        SpannedValue::CellPath { val, .. } => toml::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(toml::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(toml::Value::Integer(*val as i64)),
                })
                .collect::<Result<Vec<toml::Value>, ShellError>>()?,
        ),
        SpannedValue::CustomValue { .. } => toml::Value::String("<Custom Value>".to_string()),
        SpannedValue::MatchPattern { .. } => toml::Value::String("<Match Pattern>".to_string()),
    })
}

fn toml_list(
    engine_state: &EngineState,
    input: &[SpannedValue],
) -> Result<Vec<toml::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(helper(engine_state, value)?);
    }

    Ok(out)
}

fn toml_into_pipeline_data(
    toml_value: &toml::Value,
    value_type: Type,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match toml::to_string(&toml_value) {
        Ok(serde_toml_string) => Ok(SpannedValue::String {
            val: serde_toml_string,
            span,
        }
        .into_pipeline_data()),
        _ => Ok(SpannedValue::Error {
            error: Box::new(ShellError::CantConvert {
                to_type: "TOML".into(),
                from_type: value_type.to_string(),
                span,
                help: None,
            }),
        }
        .into_pipeline_data()),
    }
}

fn value_to_toml_value(
    engine_state: &EngineState,
    v: &SpannedValue,
    head: Span,
) -> Result<toml::Value, ShellError> {
    match v {
        SpannedValue::Record { .. } => helper(engine_state, v),
        // Propagate existing errors
        SpannedValue::Error { error } => Err(*error.clone()),
        _ => Err(ShellError::UnsupportedInput(
            format!("{:?} is not valid top-level TOML", v.get_type()),
            "value originates from here".into(),
            head,
            v.expect_span(),
        )),
    }
}

fn to_toml(
    engine_state: &EngineState,
    input: PipelineData,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(span);

    let toml_value = value_to_toml_value(engine_state, &value, span)?;
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

        let engine_state = EngineState::new();

        let mut m = indexmap::IndexMap::new();
        m.insert("rust".to_owned(), SpannedValue::test_string("editor"));
        m.insert("is".to_owned(), SpannedValue::nothing(Span::test_data()));
        m.insert(
            "features".to_owned(),
            SpannedValue::List {
                vals: vec![
                    SpannedValue::test_string("hello"),
                    SpannedValue::test_string("array"),
                ],
                span: Span::test_data(),
            },
        );
        let tv = value_to_toml_value(
            &engine_state,
            &SpannedValue::from(Spanned {
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
        //
        // Negative Tests
        //
        value_to_toml_value(
            &engine_state,
            &SpannedValue::test_string("not_valid"),
            Span::test_data(),
        )
        .expect_err("Expected non-valid toml (String) to cause error!");
        value_to_toml_value(
            &engine_state,
            &SpannedValue::List {
                vals: vec![SpannedValue::test_string("1")],
                span: Span::test_data(),
            },
            Span::test_data(),
        )
        .expect_err("Expected non-valid toml (Table) to cause error!");
    }
}
