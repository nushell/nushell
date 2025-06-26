use core::fmt::Write;
use nu_engine::get_columns;
use nu_protocol::{Range, ShellError, Span, Value, engine::EngineState};
use nu_utils::{ObviousFloat, escape_quote_string, needs_quoting};

/// control the way Nushell [`Value`] is converted to NUON data
pub enum ToStyle {
    /// no extra indentation
    ///
    /// `{ a: 1, b: 2 }` will be converted to `{a: 1, b: 2}`
    Default,
    /// no white space at all
    ///
    /// `{ a: 1, b: 2 }` will be converted to `{a:1,b:2}`
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
pub fn to_nuon(
    engine_state: &EngineState,
    input: &Value,
    style: ToStyle,
    span: Option<Span>,
    serialize_types: bool,
) -> Result<String, ShellError> {
    let span = span.unwrap_or(Span::unknown());

    let indentation = match style {
        ToStyle::Default => None,
        ToStyle::Raw => Some("".to_string()),
        ToStyle::Tabs(t) => Some("\t".repeat(t)),
        ToStyle::Spaces(s) => Some(" ".repeat(s)),
    };

    let res = value_to_string(
        engine_state,
        input,
        span,
        0,
        indentation.as_deref(),
        serialize_types,
    )?;

    Ok(res)
}

fn value_to_string(
    engine_state: &EngineState,
    v: &Value,
    span: Span,
    depth: usize,
    indent: Option<&str>,
    serialize_types: bool,
) -> Result<String, ShellError> {
    let (nl, sep, kv_sep) = get_true_separators(indent);
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
        Value::Closure { val, .. } => {
            if serialize_types {
                Ok(escape_quote_string(
                    &val.coerce_into_string(engine_state, span)?,
                ))
            } else {
                Err(ShellError::UnsupportedInput {
                    msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
                    input: "value originates from here".into(),
                    msg_span: span,
                    input_span: v.span(),
                })
            }
        }
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
        Value::Float { val, .. } => Ok(ObviousFloat(*val).to_string()),
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
                                engine_state,
                                val,
                                span,
                                depth + 2,
                                indent,
                                serialize_types,
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
                        value_to_string_without_quotes(
                            engine_state,
                            val,
                            span,
                            depth + 1,
                            indent,
                            serialize_types
                        )?
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
            Range::FloatRange(range) => Ok(range.to_string()),
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
                    "{idt_po}{col}:{kv_sep}{}",
                    value_to_string_without_quotes(
                        engine_state,
                        val,
                        span,
                        depth + 1,
                        indent,
                        serialize_types
                    )?
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

/// Converts the provided indent into three types of separator:
/// - New line separators
/// - Inline separator
/// - Key-value separators inside Records
fn get_true_separators(indent: Option<&str>) -> (String, String, String) {
    match indent {
        Some("") => ("".to_string(), "".to_string(), "".to_string()),
        Some(_) => ("\n".to_string(), "".to_string(), " ".to_string()),
        None => ("".to_string(), " ".to_string(), " ".to_string()),
    }
}

fn value_to_string_without_quotes(
    engine_state: &EngineState,
    v: &Value,
    span: Span,
    depth: usize,
    indent: Option<&str>,
    serialize_types: bool,
) -> Result<String, ShellError> {
    match v {
        Value::String { val, .. } => Ok({
            if needs_quoting(val) {
                escape_quote_string(val)
            } else {
                val.clone()
            }
        }),
        _ => value_to_string(engine_state, v, span, depth, indent, serialize_types),
    }
}
