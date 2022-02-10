use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct ToNuon;

impl Command for ToNuon {
    fn name(&self) -> &str {
        "to nuon"
    }

    fn signature(&self) -> Signature {
        Signature::build("to nuon")
            .switch("raw", "remove all of the whitespace", Some('r'))
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts table data into Nuon (Nushell Object Notation) text."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        Ok(Value::String {
            val: to_nuon(call, input)?,
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description:
                "Outputs an unformatted JSON string representing the contents of this table",
            example: "[1 2 3] | to nuon",
            result: Some(Value::test_string("[\n  1,\n  2,\n  3\n]")),
        }]
    }
}

fn value_to_string(v: &Value, span: Span) -> Result<String, ShellError> {
    match v {
        Value::Binary { .. } => Err(ShellError::UnsupportedInput("binary".into(), span)),
        Value::Block { .. } => Err(ShellError::UnsupportedInput("block".into(), span)),
        Value::Bool { val, .. } => {
            if *val {
                Ok("$true".to_string())
            } else {
                Ok("$false".to_string())
            }
        }
        Value::CellPath { .. } => Err(ShellError::UnsupportedInput("cellpath".to_string(), span)),
        Value::CustomValue { .. } => Err(ShellError::UnsupportedInput("custom".to_string(), span)),
        Value::Date { .. } => Err(ShellError::UnsupportedInput("date".to_string(), span)),
        Value::Duration { val, .. } => Ok(format!("{}ns", *val)),
        Value::Error { .. } => Err(ShellError::UnsupportedInput("error".to_string(), span)),
        Value::Filesize { val, .. } => Ok(format!("{}b", *val)),
        Value::Float { val, .. } => Ok(format!("{}", *val)),
        Value::Int { val, .. } => Ok(format!("{}", *val)),
        Value::List { vals, .. } => {
            let mut collection = vec![];
            for val in vals {
                collection.push(value_to_string(val, span)?);
            }
            Ok(format!("[{}]", collection.join(", ")))
        }
        Value::Nothing { .. } => Ok("$nothing".to_string()),
        Value::Range { val, .. } => Ok(format!(
            "{}..{}{}",
            value_to_string(&val.from, span)?,
            if val.inclusion == RangeInclusion::RightExclusive {
                "<"
            } else {
                ""
            },
            value_to_string(&val.to, span)?
        )),
        Value::Record { cols, vals, .. } => {
            let mut collection = vec![];
            for (col, val) in cols.iter().zip(vals) {
                collection.push(format!("\"{}\": {}", col, value_to_string(val, span)?));
            }
            Ok(format!("{{{}}}", collection.join(", ")))
        }
        Value::String { val, .. } => Ok(format!("\"{}\"", val)),
    }
}

fn to_nuon(call: &Call, input: PipelineData) -> Result<String, ShellError> {
    let v = input.into_value(call.head);

    value_to_string(&v, call.head)
}
