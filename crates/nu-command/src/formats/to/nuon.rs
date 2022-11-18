use core::fmt::Write;
use fancy_regex::Regex;
use nu_engine::get_columns;
use nu_parser::escape_quote_string;
use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Type, Value,
};
use once_cell::sync::Lazy;

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
                        "could not convert binary to string".into(),
                        span,
                    ));
                }
            }
            Ok(format!("0x[{}]", s))
        }
        Value::Block { .. } => Err(ShellError::UnsupportedInput(
            "blocks are currently not nuon-compatible".into(),
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
            "cellpaths are currently not nuon-compatible".to_string(),
            span,
        )),
        Value::CustomValue { .. } => Err(ShellError::UnsupportedInput(
            "customs are currently not nuon-compatible".to_string(),
            span,
        )),
        Value::Date { val, .. } => Ok(val.to_rfc3339()),
        // FIXME: make duratiobs use the shortest lossless representation.
        Value::Duration { val, .. } => Ok(format!("{}ns", *val)),
        Value::Error { .. } => Err(ShellError::UnsupportedInput(
            "errors are currently not nuon-compatible".to_string(),
            span,
        )),
        // FIXME: make filesizes use the shortest lossless representation.
        Value::Filesize { val, .. } => Ok(format!("{}b", *val)),
        Value::Float { val, .. } => {
            // This serialises these as 'nan', 'inf' and '-inf', respectively.
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
        Value::Nothing { .. } => Ok("null".to_string()),
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
        // All strings outside data structures are quoted because they are in 'command position'
        // (could be mistaken for commands by the Nu parser)
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

// This hits, in order:
// • Any character of []:`{}#'";()|$,
// • Any digit (\d)
// • Any whitespace (\s)
// • Case-insensitive sign-insensitive float "keywords" inf, infinity and nan.
static NEEDS_QUOTES_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[\[\]:`\{\}#'";\(\)\|\$,\d\s]|(?i)^[+\-]?(inf(inity)?|nan)$"#)
        .expect("internal error: NEEDS_QUOTES_REGEX didn't compile")
});

fn needs_quotes(string: &str) -> bool {
    // These are case-sensitive keywords
    match string {
        "true" | "false" | "null" => return true,
        _ => (),
    };
    // All other cases are handled here
    NEEDS_QUOTES_REGEX.is_match(string).unwrap_or(false)
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
