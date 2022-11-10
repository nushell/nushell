use core::fmt::Write;
use nu_engine::get_columns;
use nu_parser::escape_quote_string;
use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ToNuon;

impl Command for ToNuon {
    fn name(&self) -> &str {
        "to nuon"
    }

    fn signature(&self) -> Signature {
        Signature::build("to nuon")
            .input_output_types(vec![(Type::Any, Type::String)])
            .category(Category::Experimental)
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
            description: "Outputs a nuon string representing the contents of this list",
            example: "[1 2 3] | to nuon",
            result: Some(Value::test_string("[1, 2, 3]")),
        }]
    }
}

fn value_to_string(v: &Value, span: Span) -> Result<String, ShellError> {
    match v {
        Value::Binary { val, .. } => {
            let mut s = String::with_capacity(2 * val.len());
            for byte in val {
                if write!(s, "{:02X}", byte).is_err() {
                    return Err(ShellError::UnsupportedInput(
                        "binary could not translate to string".into(),
                        span,
                    ));
                }
            }
            Ok(format!("0x[{}]", s))
        }
        Value::Block { .. } => Err(ShellError::UnsupportedInput(
            "block not supported".into(),
            span,
        )),
        Value::Closure { .. } => Err(ShellError::UnsupportedInput(
            "closure not supported".into(),
            span,
        )),
        Value::Bool { val, .. } => {
            if *val {
                Ok("true".to_string())
            } else {
                Ok("false".to_string())
            }
        }
        Value::CellPath { .. } => Err(ShellError::UnsupportedInput(
            "cellpath not supported".to_string(),
            span,
        )),
        Value::CustomValue { .. } => Err(ShellError::UnsupportedInput(
            "custom not supported".to_string(),
            span,
        )),
        Value::Date { val, .. } => Ok(val.to_rfc3339()),
        Value::Duration { val, .. } => Ok(format!("{}ns", *val)),
        Value::Error { .. } => Err(ShellError::UnsupportedInput(
            "error not supported".to_string(),
            span,
        )),
        Value::Filesize { val, .. } => Ok(format!("{}b", *val)),
        Value::Float { val, .. } => {
            if &val.round() == val
                && val != &f64::NAN
                && val != &f64::INFINITY
                && val != &f64::NEG_INFINITY
            {
                Ok(format!("{}.0", *val))
            } else {
                Ok(format!("{}", *val))
            }
        }
        Value::Int { val, .. } => Ok(format!("{}", *val)),
        Value::List { vals, .. } => {
            let headers = get_columns(vals);
            if !headers.is_empty() && vals.iter().all(|x| x.columns() == headers) {
                // Table output
                let headers: Vec<String> = headers
                    .iter()
                    .map(|string| {
                        if needs_quotes(string) {
                            format!("\"{}\"", string)
                        } else {
                            string.to_string()
                        }
                    })
                    .collect();
                let headers_output = headers.join(", ");

                let mut table_output = vec![];
                for val in vals {
                    let mut row = vec![];

                    if let Value::Record { vals, .. } = val {
                        for val in vals {
                            row.push(value_to_string_without_quotes(val, span)?);
                        }
                    }

                    table_output.push(row.join(", "));
                }

                Ok(format!(
                    "[[{}]; [{}]]",
                    headers_output,
                    table_output.join("], [")
                ))
            } else {
                let mut collection = vec![];
                for val in vals {
                    collection.push(value_to_string_without_quotes(val, span)?);
                }
                Ok(format!("[{}]", collection.join(", ")))
            }
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
                collection.push(if needs_quotes(col) {
                    format!(
                        "\"{}\": {}",
                        col,
                        value_to_string_without_quotes(val, span)?
                    )
                } else {
                    format!("{}: {}", col, value_to_string_without_quotes(val, span)?)
                });
            }
            Ok(format!("{{{}}}", collection.join(", ")))
        }
        Value::String { val, .. } => Ok(escape_quote_string(val)),
    }
}

fn value_to_string_without_quotes(v: &Value, span: Span) -> Result<String, ShellError> {
    match v {
        Value::String { val, .. } => Ok({
            if needs_quotes(val) {
                escape_quote_string(val)
            } else {
                val.clone()
            }
        }),
        _ => value_to_string(v, span),
    }
}

fn to_nuon(call: &Call, input: PipelineData) -> Result<String, ShellError> {
    let v = input.into_value(call.head);

    value_to_string(&v, call.head)
}

fn needs_quotes(string: &str) -> bool {
    string.contains(' ')
        || string.contains('[')
        || string.contains(']')
        || string.contains(':')
        || string.contains('`')
        || string.contains('{')
        || string.contains('}')
        || string.contains('#')
        || string.contains('\'')
        || string.contains(';')
        || string.contains('(')
        || string.contains(')')
        || string.contains('|')
        || string.contains('$')
        || string.contains(',')
        || string.contains('\t')
        || string.contains('\n')
        || string.contains('\r')
        || string.contains('\"')
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ToNuon;
        use crate::test_examples;
        test_examples(ToNuon {})
    }
}
