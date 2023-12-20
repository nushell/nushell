use core::fmt::Write;
use fancy_regex::Regex;
use nu_engine::get_columns;
use nu_engine::CallExt;
use nu_parser::escape_quote_string;
use nu_protocol::ast::{Call, RangeInclusion};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
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
            .switch(
                "raw",
                "remove all of the whitespace (default behaviour and overwrites -i and -t)",
                Some('r'),
            )
            .named(
                "indent",
                SyntaxShape::Number,
                "specify indentation width",
                Some('i'),
            )
            .named(
                "tabs",
                SyntaxShape::Number,
                "specify indentation tab quantity",
                Some('t'),
            )
            .category(Category::Formats)
    }

    fn usage(&self) -> &str {
        "Converts table data into Nuon (Nushell Object Notation) text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw = call.has_flag("raw");
        let use_tabs = call.has_flag("tabs");
        let use_indent = call.has_flag("indent");

        let span = call.head;
        let value = input.into_value(span);

        let nuon_result = if raw {
            value_to_string(&value, span, 0, None)
        } else if use_tabs {
            let tab_count: usize = call.get_flag(engine_state, stack, "tabs")?.unwrap_or(1);
            value_to_string(&value, span, 0, Some(&"\t".repeat(tab_count)))
        } else if use_indent {
            let indent: usize = call.get_flag(engine_state, stack, "indent")?.unwrap_or(2);
            value_to_string(&value, span, 0, Some(&" ".repeat(indent)))
        } else {
            value_to_string(&value, span, 0, None)
        };

        match nuon_result {
            Ok(serde_nuon_string) => {
                Ok(Value::string(serde_nuon_string, span).into_pipeline_data())
            }
            _ => Ok(Value::error(
                ShellError::CantConvert {
                    to_type: "NUON".into(),
                    from_type: value.get_type().to_string(),
                    span,
                    help: None,
                },
                span,
            )
            .into_pipeline_data()),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs a NUON string representing the contents of this list, compact by default",
                example: "[1 2 3] | to nuon",
                result: Some(Value::test_string("[1, 2, 3]"))
            },
            Example {
                description: "Outputs a NUON array of ints, with pretty indentation",
                example: "[1 2 3] | to nuon --indent 2",
                result: Some(Value::test_string("[\n  1,\n  2,\n  3\n]")),
            },
            Example {
                description: "Overwrite any set option with --raw",
                example: "[1 2 3] | to nuon --indent 2 --raw",
                result: Some(Value::test_string("[1, 2, 3]"))
            },
            Example {
                description: "A more complex record with multiple data types",
                example: "{date: 2000-01-01, data: [1 [2 3] 4.56]} | to nuon --indent 2",
                result: Some(Value::test_string("{\n  date: 2000-01-01T00:00:00+00:00,\n  data: [\n    1,\n    [\n      2,\n      3\n    ],\n    4.56\n  ]\n}"))
            }
        ]
    }
}

pub fn value_to_string(
    v: &Value,
    span: Span,
    depth: usize,
    indent: Option<&str>,
) -> Result<String, ShellError> {
    let (nl, sep) = get_true_separators(indent);
    let idt = get_true_indentation(depth, indent);
    let idt_po = get_true_indentation(depth + 1, indent);
    let idt_pt = get_true_indentation(depth + 2, indent);

    match v {
        Value::Binary { val, .. } => {
            let mut s = String::with_capacity(2 * val.len());
            for byte in val {
                if write!(s, "{byte:02X}").is_err() {
                    return Err(ShellError::UnsupportedInput {
                        msg: "could not convert binary to string".into(),
                        input: "value originates from here".into(),
                        msg_span: span,
                        input_span: v.span(),
                    });
                }
            }
            Ok(format!("0x[{s}]"))
        }
        Value::Block { .. } => Err(ShellError::UnsupportedInput {
            msg: "blocks are currently not nuon-compatible".into(),
            input: "value originates from here".into(),
            msg_span: span,
            input_span: v.span(),
        }),
        Value::Closure { .. } => Err(ShellError::UnsupportedInput {
            msg: "closures are currently not nuon-compatible".into(),
            input: "value originates from here".into(),
            msg_span: span,
            input_span: v.span(),
        }),
        Value::Bool { val, .. } => {
            if *val {
                Ok("true".to_string())
            } else {
                Ok("false".to_string())
            }
        }
        Value::CellPath { .. } => Err(ShellError::UnsupportedInput {
            msg: "cell-paths are currently not nuon-compatible".to_string(),
            input: "value originates from here".into(),
            msg_span: span,
            input_span: v.span(),
        }),
        Value::CustomValue { .. } => Err(ShellError::UnsupportedInput {
            msg: "custom values are currently not nuon-compatible".to_string(),
            input: "value originates from here".into(),
            msg_span: span,
            input_span: v.span(),
        }),
        Value::Date { val, .. } => Ok(val.to_rfc3339()),
        // FIXME: make durations use the shortest lossless representation.
        Value::Duration { val, .. } => Ok(format!("{}ns", *val)),
        // Propagate existing errors
        Value::Error { error, .. } => Err(*error.clone()),
        // FIXME: make filesizes use the shortest lossless representation.
        Value::Filesize { val, .. } => Ok(format!("{}b", *val)),
        Value::Float { val, .. } => {
            // This serialises these as 'nan', 'inf' and '-inf', respectively.
            if &val.round() == val && val.is_finite() {
                Ok(format!("{}.0", *val))
            } else {
                Ok(format!("{}", *val))
            }
        }
        Value::Int { val, .. } => Ok(format!("{}", *val)),
        Value::List { vals, .. } => {
            let headers = get_columns(vals);
            if !headers.is_empty() && vals.iter().all(|x| x.columns().eq(headers.iter())) {
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

                    if let Value::Record { val, .. } = val {
                        for val in val.values() {
                            row.push(value_to_string_without_quotes(
                                val,
                                span,
                                depth + 2,
                                indent,
                            )?);
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
                        value_to_string_without_quotes(val, span, depth + 1, indent,)?
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
            value_to_string(&val.from, span, depth + 1, indent)?,
            if val.inclusion == RangeInclusion::RightExclusive {
                "<"
            } else {
                ""
            },
            value_to_string(&val.to, span, depth + 1, indent)?
        )),
        Value::Record { val, .. } => {
            let mut collection = vec![];
            for (col, val) in val {
                collection.push(if needs_quotes(col) {
                    format!(
                        "{idt_po}\"{}\": {}",
                        col,
                        value_to_string_without_quotes(val, span, depth + 1, indent)?
                    )
                } else {
                    format!(
                        "{idt_po}{}: {}",
                        col,
                        value_to_string_without_quotes(val, span, depth + 1, indent)?
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
            value_to_string(&collected, span, depth + 1, indent)
        }
        // All strings outside data structures are quoted because they are in 'command position'
        // (could be mistaken for commands by the Nu parser)
        Value::String { val, .. } => Ok(escape_quote_string(val)),
    }
}

fn get_true_indentation(depth: usize, indent: Option<&str>) -> String {
    match indent {
        Some(i) => i.repeat(depth),
        None => "".to_string(),
    }
}

fn get_true_separators(indent: Option<&str>) -> (String, String) {
    match indent {
        Some(_) => ("\n".to_string(), "".to_string()),
        None => ("".to_string(), " ".to_string()),
    }
}

fn value_to_string_without_quotes(
    v: &Value,
    span: Span,
    depth: usize,
    indent: Option<&str>,
) -> Result<String, ShellError> {
    match v {
        Value::String { val, .. } => Ok({
            if needs_quotes(val) {
                escape_quote_string(val)
            } else {
                val.clone()
            }
        }),
        _ => value_to_string(v, span, depth, indent),
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
        use super::ToNuon;
        use crate::test_examples;
        test_examples(ToNuon {})
    }
}
