use core::fmt::Write;

use nu_engine::get_columns;
use nu_protocol::{Range, ShellError, Span, Value};
use nu_utils::{escape_quote_string, needs_quoting};

use std::ops::Bound;

/// control the way Nushell [`Value`] is converted to NUON data
pub enum ToStyle {
    /// no indentation at all
    ///
    /// `{ a: 1, b: 2 }` will be converted to `{a: 1, b: 2}`
    Raw,
    #[allow(clippy::tabs_in_doc_comments)]
    /// tabulation-based indentation
    ///
    /// using 2 as the variant value, `{ a: 1, b: 2 }` will be converted to
    /// ```text
    /// {
    /// 		a: 1,
    /// 		b: 2
    /// }
    /// ```
    Tabs(usize),
    /// space-based indentation
    ///
    /// using 3 as the variant value, `{ a: 1, b: 2 }` will be converted to
    /// ```text
    /// {
    ///    a: 1,
    ///    b: 2
    /// }
    /// ```
    Spaces(usize),
}

/// convert an actual Nushell [`Value`] to a raw string representation of the NUON data
///
// WARNING: please leave the following two trailing spaces, they matter for the documentation
// formatting
/// > **Note**
/// > a [`Span`] can be passed to [`to_nuon`] if there is context available to the caller, e.g. when
/// > using this function in a command implementation such as [`to nuon`](https://www.nushell.sh/commands/docs/to_nuon.html).
///
/// also see [`super::from_nuon`] for the inverse operation
pub fn to_nuon(input: &Value, style: ToStyle, span: Option<Span>) -> Result<String, ShellError> {
    let span = span.unwrap_or(Span::unknown());

    let indentation = match style {
        ToStyle::Raw => None,
        ToStyle::Tabs(t) => Some("\t".repeat(t)),
        ToStyle::Spaces(s) => Some(" ".repeat(s)),
    };

    let res = value_to_string(input, span, 0, indentation.as_deref())?;

    Ok(res)
}

fn value_to_string(
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
        Value::CellPath { val, .. } => Ok(val.to_string()),
        Value::Custom { .. } => Err(ShellError::UnsupportedInput {
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
        Value::Filesize { val, .. } => Ok(format!("{}b", val.get())),
        Value::Float { val, .. } => {
            // This serialises these as 'nan', 'inf' and '-inf', respectively.
            if &val.round() == val && val.is_finite() {
                Ok(format!("{}.0", *val))
            } else {
                Ok(val.to_string())
            }
        }
        Value::Int { val, .. } => Ok(val.to_string()),
        Value::List { vals, .. } => {
            let headers = get_columns(vals);
            if !headers.is_empty() && vals.iter().all(|x| x.columns().eq(headers.iter())) {
                // Table output
                let headers: Vec<String> = headers
                    .iter()
                    .map(|string| {
                        let string = if needs_quoting(string) {
                            &escape_quote_string(string)
                        } else {
                            string
                        };
                        format!("{idt}{string}")
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
        Value::Range { val, .. } => match **val {
            Range::IntRange(range) => Ok(range.to_string()),
            Range::FloatRange(range) => {
                let start =
                    value_to_string(&Value::float(range.start(), span), span, depth + 1, indent)?;
                match range.end() {
                    Bound::Included(end) => Ok(format!(
                        "{}..{}",
                        start,
                        value_to_string(&Value::float(end, span), span, depth + 1, indent)?
                    )),
                    Bound::Excluded(end) => Ok(format!(
                        "{}..<{}",
                        start,
                        value_to_string(&Value::float(end, span), span, depth + 1, indent)?
                    )),
                    Bound::Unbounded => Ok(format!("{start}..",)),
                }
            }
        },
        Value::Record { val, .. } => {
            let mut collection = vec![];
            for (col, val) in &**val {
                let col = if needs_quoting(col) {
                    &escape_quote_string(col)
                } else {
                    col
                };
                collection.push(format!(
                    "{idt_po}{col}: {}",
                    value_to_string_without_quotes(val, span, depth + 1, indent)?
                ));
            }
            Ok(format!(
                "{{{nl}{}{nl}{idt}}}",
                collection.join(&format!(",{sep}{nl}"))
            ))
        }
        // All strings outside data structures are quoted because they are in 'command position'
        // (could be mistaken for commands by the Nu parser)
        Value::String { val, .. } => Ok(escape_quote_string(val)),
        Value::Glob { val, .. } => Ok(escape_quote_string(val)),
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
            if needs_quoting(val) {
                escape_quote_string(val)
            } else {
                val.clone()
            }
        }),
        _ => value_to_string(v, span, depth, indent),
    }
}
