use core::fmt::Write;
use fancy_regex::Regex;
use nu_engine::get_columns;
use nu_parser::escape_quote_string;
use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Type, Value,
};
use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct ToParsable;

// credit: code gleefully adapted from `to nuon`

impl Command for ToParsable {
    fn name(&self) -> &str {
        "to parsable"
    }

    fn signature(&self) -> Signature {
        Signature::build("to parsable")
            .input_output_types(vec![(Type::Any, Type::String)])
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts objects into strings that are parsable by Nushell and colorable by nu-highlight."
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let value = input.into_value(span);

        match value_to_string(&value, span) {
            Ok(parsable_string) => Ok(Value::String {
                val: parsable_string,
                span,
            }
            .into_pipeline_data()),
            _ => Ok(Value::Error {
                error: Box::new(ShellError::CantConvert {
                    to_type: "parsable string".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                }),
            }
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Types that would normally produce unparsable strings",
                example: r#"{date: 2000-01-01, data: ["embedded spaces"  23b ]}"#,
                result: Some(Value::Record {
                    cols: vec!["date".to_string(), "data".to_string()],
                    vals: vec![
                        Value::test_string("23 years ago"),
                        Value::test_list(vec![
                            Value::test_string("embedded spaces"),
                            Value::test_filesize(23),
                        ]),
                    ],
                    span,
                }),
            },
            Example {
                description: "Types producing parsable and thus colorizable strings",
                example: r#"{date: 2000-01-01, data: ["embedded spaces"  23b ]}"#,
                result: Some(Value::test_string(
                    "{date: 2000-01-01T00:00:00+00:00, data: ['embedded spaces' 23b]}",
                )),
            },
            Example {
                description: "Producing nice table literal from ragged list of records",
                example: r#"[{a:10 b:"embedded spaces"}, { a:10, c:2023-10-03 }, {b:20b}, {c:"apple of my eye"}] "#,
                result: None,
            },
        ]
    }
}

pub fn value_to_string(v: &Value, span: Span) -> Result<String, ShellError> {
    let idt = "";
    let idt_po = "";
    let idt_pt = "";
    let nl = "";
    let sep = "";

    match v {
        // Propagate existing errors
        Value::Error { error } => Err(*error.clone()),

        Value::Binary { val, .. } => {
            let mut s = String::with_capacity(2 * val.len());
            for byte in val {
                if write!(s, "{byte:02X}").is_err() {
                    return Err(ShellError::UnsupportedInput(
                        "could not convert binary to string".into(),
                        "value originates from here".into(),
                        span,
                        v.expect_span(),
                    ));
                }
            }
            Ok(format!("0x[{s}]"))
        }
        Value::Bool { val, .. } => {
            if *val {
                Ok("true".to_string())
            } else {
                Ok("false".to_string())
            }
        }
        Value::Date { val, .. } => Ok(val.to_rfc3339()),
        // FIXME: make durations use the shortest lossless representation.
        Value::Duration { val, .. } => Ok(format!("{}ns", *val)),
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
                            format!("{idt}\"{string}\"")
                        } else {
                            format!("{idt}{string}")
                        }
                    })
                    .collect();
                let headers_output = headers.join(&format!(",{sep}{nl}{idt_pt}"));

                let mut table_output = vec![];
                for val in vals {
                    let mut row = vec![];

                    if let Value::Record { vals, .. } = val {
                        for val in vals {
                            row.push(value_to_string_without_quotes(val, span)?);
                        }
                    }

                    table_output.push(row.join(&format!(",{sep}{nl}{idt_pt}")));
                }

                Ok(format!(
                    "[{nl}{idt_po}[{nl}{idt_pt}{}{nl}{idt_po}];{sep}{nl}{idt_po}[{nl}{idt_pt}{}{nl}{idt_po}]{nl}{idt}]",
                    headers_output,
                    table_output.join(&format!("{nl}{idt_po}],{sep}{nl}{idt_po}[{nl}{idt_pt}"))
                ))
            } else {
                let mut collection = vec![];
                for val in vals {
                    collection.push(format!(
                        "{idt_po}{}",
                        value_to_string_without_quotes(val, span,)?
                    ));
                }
                Ok(format!(
                    "[{nl}{}{nl}{idt}]",
                    collection.join(&format!(",{sep}{nl}"))
                ))
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
                        "{idt_po}\"{}\": {}",
                        col,
                        value_to_string_without_quotes(val, span)?
                    )
                } else {
                    format!(
                        "{idt_po}{}: {}",
                        col,
                        value_to_string_without_quotes(val, span)?
                    )
                });
            }
            Ok(format!(
                "{{{nl}{}{nl}{idt}}}",
                collection.join(&format!(",{sep}{nl}"))
            ))
        }
        Value::LazyRecord { val, .. } => {
            let collected = val.collect()?;
            value_to_string(&collected, span)
        }
        // All strings outside data structures are quoted because they are in 'command position'
        // (could be mistaken for commands by the Nu parser)
        Value::String { val, .. } => Ok(escape_quote_string(val)),

        _ => Err(ShellError::UnsupportedInput(
            format!("{} not parsable-compatible", v.get_type()),
            "value originates from here".into(),
            span,
            v.expect_span(),
        )),
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
    if string.is_empty() {
        return true;
    }
    // These are case-sensitive keywords
    match string {
        // `true`/`false`/`null` are active keywords in JSON and NUON
        // `&&` is denied by the nu parser for diagnostics reasons
        // (https://github.com/nushell/nushell/pull/7241)
        // TODO: remove the extra check in the nuon codepath
        "true" | "false" | "null" | "&&" => return true,
        _ => (),
    };
    // All other cases are handled here
    NEEDS_QUOTES_REGEX.is_match(string).unwrap_or(false)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::ToParsable;
        use crate::test_examples;
        test_examples(ToParsable {})
    }
}
