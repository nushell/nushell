use nu_protocol::{
    DEFAULT_ERROR_CONTEXT, Filesize, IntoValue, Range, Record, ShellError, Span, Type, Unit, Value,
    ast::{Expr, Expression, ListItem, RecordItem},
    engine::{EngineState, StateWorkingSet},
    shell_error::generic::GenericError,
    truncated_source_window,
};
use std::{borrow::Cow, sync::Arc};

fn truncated_nuon_error(src: &str, err_span: Span, msg: impl Into<String>) -> ShellError {
    let (src, span) = truncated_source_window(src, err_span, DEFAULT_ERROR_CONTEXT);
    ShellError::OutsideSpannedLabeledError {
        src,
        error: "Error when loading".into(),
        msg: msg.into(),
        span,
    }
}

/// convert a raw string representation of NUON data to an actual Nushell [`Value`]
///
// WARNING: please leave the following two trailing spaces, they matter for the documentation
// formatting
/// > **Note**
/// > [`Span`] can be passed to [`from_nuon`] if there is context available to the caller, e.g. when
/// > using this function in a command implementation such as
/// > [`from nuon`](https://www.nushell.sh/commands/docs/from_nuon.html).
///
/// also see [`super::to_nuon`] for the inverse operation
pub fn from_nuon(input: &str, span: Option<Span>) -> Result<Value, ShellError> {
    let mut engine_state = EngineState::default();
    // NOTE: the parser needs `$env.PWD` to be set, that's a know _API issue_ with the
    // [`EngineState`]
    engine_state.add_env_var("PWD".to_string(), Value::string("", Span::unknown()));
    let mut working_set = StateWorkingSet::new(&engine_state);

    let mut block = nu_parser::parse(&mut working_set, None, input.as_bytes(), false);

    if let Some(pipeline) = block.pipelines.get(1) {
        if let Some(element) = pipeline.elements.first() {
            let (src, label_span) =
                truncated_source_window(input, element.expr.span, DEFAULT_ERROR_CONTEXT);
            return Err(ShellError::Generic(
                make_generic_error(
                    span,
                    "error when loading nuon text",
                    "could not load nuon text",
                )
                .with_inner([ShellError::OutsideSpannedLabeledError {
                    src,
                    error: "error when loading".into(),
                    msg: "excess values when loading".into(),
                    span: label_span,
                }]),
            ));
        } else {
            return Err(ShellError::Generic(
                make_generic_error(
                    span,
                    "error when loading nuon text",
                    "could not load nuon text",
                )
                .with_inner([ShellError::Generic(make_generic_error(
                    span,
                    "error when loading",
                    "excess values when loading",
                ))]),
            ));
        }
    }

    let expr = if block.pipelines.is_empty() {
        Expression::new(
            &mut working_set,
            Expr::Nothing,
            span.unwrap_or(Span::unknown()),
            Type::Nothing,
        )
    } else {
        let mut pipeline = Arc::make_mut(&mut block).pipelines.remove(0);

        if let Some(expr) = pipeline.elements.get(1) {
            let (src, label_span) =
                truncated_source_window(input, expr.expr.span, DEFAULT_ERROR_CONTEXT);
            return Err(ShellError::Generic(
                make_generic_error(
                    span,
                    "error when loading nuon text",
                    "could not load nuon text",
                )
                .with_inner([ShellError::OutsideSpannedLabeledError {
                    src,
                    error: "error when loading".into(),
                    msg: "detected a pipeline in nuon file".into(),
                    span: label_span,
                }]),
            ));
        }

        if pipeline.elements.is_empty() {
            Expression::new(
                &mut working_set,
                Expr::Nothing,
                span.unwrap_or(Span::unknown()),
                Type::Nothing,
            )
        } else {
            pipeline.elements.remove(0).expr
        }
    };

    if let Some(err) = working_set.parse_errors.first() {
        let (src, label_span) = truncated_source_window(input, err.span(), DEFAULT_ERROR_CONTEXT);
        return Err(ShellError::Generic(
            make_generic_error(
                span,
                "error when parsing nuon text",
                "could not parse nuon text",
            )
            .with_inner([ShellError::OutsideSpannedLabeledError {
                src,
                error: "error when parsing".into(),
                msg: err.to_string(),
                span: label_span,
            }]),
        ));
    }

    let value = convert_to_value(expr, span.unwrap_or(Span::unknown()), input)?;

    Ok(value)
}

fn make_generic_error(
    span: Option<Span>,
    error: impl Into<Cow<'static, str>>,
    msg: impl Into<Cow<'static, str>>,
) -> GenericError {
    match span {
        Some(span) => GenericError::new(error, msg, span),
        None => GenericError::new_internal(error, msg),
    }
}

/// Scale a duration head by its nanosecond factor, rejecting overflow.
///
/// The parser clamps an out-of-range literal head to `i64::MAX` and keeps the
/// unit it folded to, so `size` can already be at the limit. `checked_mul`
/// rather than `*`: a plain multiply panics in debug builds and wraps to a
/// negative duration in release ones, and the nuon spec requires an error.
/// `Unit::Day` and `Unit::Week` already worked this way.
fn duration_with_scale(
    size: i64,
    factor: i64,
    unit_name: &str,
    original_text: &str,
    span: Span,
) -> Result<Value, ShellError> {
    match size.checked_mul(factor) {
        Some(val) => Ok(Value::duration(val, span)),
        None => {
            let (src, label_span) =
                truncated_source_window(original_text, span, DEFAULT_ERROR_CONTEXT);
            Err(ShellError::OutsideSpannedLabeledError {
                src,
                error: format!("{unit_name} duration too large"),
                msg: format!("{unit_name} duration too large"),
                span: label_span,
            })
        }
    }
}

fn convert_to_value(
    expr: Expression,
    span: Span,
    original_text: &str,
) -> Result<Value, ShellError> {
    match expr.expr {
        Expr::AttributeBlock(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "attributes not supported in nuon",
        )),
        Expr::BinaryOp(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "binary operators not supported in nuon",
        )),
        Expr::UnaryNot(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "unary operators not supported in nuon",
        )),
        Expr::Block(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "blocks not supported in nuon",
        )),
        Expr::Closure(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "closures not supported in nuon",
        )),
        Expr::Binary(val) => Ok(Value::binary(val, span)),
        Expr::Bool(val) => Ok(Value::bool(val, span)),
        Expr::Call(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "calls not supported in nuon",
        )),
        Expr::CellPath(val) => Ok(Value::cell_path(val, span)),
        Expr::DateTime(dt) => Ok(Value::date(dt, span)),
        Expr::ExternalCall(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "calls not supported in nuon",
        )),
        Expr::Filepath(val, _) => Ok(Value::string(val, span)),
        Expr::Directory(val, _) => Ok(Value::string(val, span)),
        Expr::Float(val) => Ok(Value::float(val, span)),
        Expr::FullCellPath(full_cell_path) => {
            if !full_cell_path.tail.is_empty() {
                Err(truncated_nuon_error(
                    original_text,
                    expr.span,
                    "subexpressions and cellpaths not supported in nuon",
                ))
            } else {
                convert_to_value(full_cell_path.head, span, original_text)
            }
        }

        Expr::Garbage => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "extra tokens in input file",
        )),
        Expr::GlobPattern(val, _) => Ok(Value::string(val, span)),
        Expr::ImportPattern(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "imports not supported in nuon",
        )),
        Expr::Overlay(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "overlays not supported in nuon",
        )),
        Expr::Int(val) => Ok(Value::int(val, span)),
        Expr::Keyword(kw) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            format!(
                "{} not supported in nuon",
                String::from_utf8_lossy(&kw.keyword)
            ),
        )),
        Expr::List(vals) => {
            let mut output = vec![];

            for item in vals {
                match item {
                    ListItem::Item(expr) => {
                        output.push(convert_to_value(expr, span, original_text)?);
                    }
                    ListItem::Spread(_, inner) => {
                        return Err(truncated_nuon_error(
                            original_text,
                            inner.span,
                            "spread operator not supported in nuon",
                        ));
                    }
                }
            }

            Ok(Value::list(output, span))
        }
        Expr::MatchBlock(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "match blocks not supported in nuon",
        )),
        Expr::Nothing => Ok(Value::nothing(span)),
        Expr::Operator(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "operators not supported in nuon",
        )),
        Expr::Range(range) => {
            let from = if let Some(f) = range.from {
                convert_to_value(f, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            let next = if let Some(s) = range.next {
                convert_to_value(s, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            let to = if let Some(t) = range.to {
                convert_to_value(t, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            Ok(Value::range(
                Range::new(from, next, to, range.operator.inclusion, expr.span)?,
                expr.span,
            ))
        }
        Expr::Record(key_vals) => {
            let mut record = Record::with_capacity(key_vals.len());
            let mut key_spans = Vec::with_capacity(key_vals.len());

            for key_val in key_vals {
                match key_val {
                    RecordItem::Pair(key, val) => {
                        let key_str = match key.expr {
                            Expr::String(key_str) => key_str,
                            _ => {
                                return Err(truncated_nuon_error(
                                    original_text,
                                    key.span,
                                    "only strings can be keys",
                                ));
                            }
                        };

                        if let Some(i) = record.index_of(&key_str) {
                            return Err(ShellError::ColumnDefinedTwice {
                                col_name: key_str,
                                second_use: key.span,
                                first_use: key_spans[i],
                            });
                        } else {
                            key_spans.push(key.span);
                            record.push(key_str, convert_to_value(val, span, original_text)?);
                        }
                    }
                    RecordItem::Spread(_, inner) => {
                        return Err(truncated_nuon_error(
                            original_text,
                            inner.span,
                            "spread operator not supported in nuon",
                        ));
                    }
                }
            }

            Ok(Value::record(record, span))
        }
        Expr::RowCondition(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "row conditions not supported in nuon",
        )),
        Expr::Signature(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "signatures not supported in nuon",
        )),
        Expr::String(s) | Expr::RawString(s) => Ok(Value::string(s.clone(), span)),
        Expr::StringInterpolation(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "string interpolation not supported in nuon",
        )),
        Expr::GlobInterpolation(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "glob interpolation not supported in nuon",
        )),
        Expr::Collect(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "`$in` not supported in nuon",
        )),
        Expr::Subexpression(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "subexpressions not supported in nuon",
        )),
        Expr::Table(mut table) => {
            let mut cols = vec![];

            let mut output = vec![];

            for key in table.columns.as_mut() {
                let key_str = match &mut key.expr {
                    Expr::String(key_str) => key_str,
                    _ => {
                        return Err(truncated_nuon_error(
                            original_text,
                            expr.span,
                            "only strings can be keys",
                        ));
                    }
                };

                if let Some(idx) = cols.iter().position(|existing| existing == key_str) {
                    return Err(ShellError::ColumnDefinedTwice {
                        col_name: key_str.clone(),
                        second_use: key.span,
                        first_use: table.columns[idx].span,
                    });
                } else {
                    cols.push(std::mem::take(key_str));
                }
            }

            for row in table.rows.into_vec() {
                if cols.len() != row.len() {
                    return Err(truncated_nuon_error(
                        original_text,
                        expr.span,
                        "table has mismatched columns",
                    ));
                }

                let record = cols
                    .iter()
                    .zip(row.into_vec())
                    .map(|(col, cell)| {
                        convert_to_value(cell, span, original_text).map(|val| (col.clone(), val))
                    })
                    .collect::<Result<_, _>>()?;

                output.push(Value::record(record, span));
            }

            Ok(Value::list(output, span))
        }
        Expr::ValueWithUnit(value) => {
            let size = match value.expr.expr {
                Expr::Int(val) => val,
                _ => {
                    return Err(truncated_nuon_error(
                        original_text,
                        expr.span,
                        "non-integer unit value",
                    ));
                }
            };

            match value.unit.item {
                Unit::Filesize(unit) => match Filesize::from_unit(size, unit) {
                    Some(val) => Ok(val.into_value(span)),
                    None => {
                        let (src, label_span) = truncated_source_window(
                            original_text,
                            expr.span,
                            DEFAULT_ERROR_CONTEXT,
                        );
                        Err(ShellError::OutsideSpannedLabeledError {
                            src,
                            error: "filesize too large".into(),
                            msg: "filesize too large".into(),
                            span: label_span,
                        })
                    }
                },

                Unit::Nanosecond => Ok(Value::duration(size, span)),
                Unit::Microsecond => {
                    duration_with_scale(size, 1000, "microsecond", original_text, span)
                }
                Unit::Millisecond => {
                    duration_with_scale(size, 1000 * 1000, "millisecond", original_text, span)
                }
                Unit::Second => {
                    duration_with_scale(size, 1000 * 1000 * 1000, "second", original_text, span)
                }
                Unit::Minute => duration_with_scale(
                    size,
                    1000 * 1000 * 1000 * 60,
                    "minute",
                    original_text,
                    span,
                ),
                Unit::Hour => duration_with_scale(
                    size,
                    1000 * 1000 * 1000 * 60 * 60,
                    "hour",
                    original_text,
                    span,
                ),
                Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => {
                        let (src, label_span) = truncated_source_window(
                            original_text,
                            expr.span,
                            DEFAULT_ERROR_CONTEXT,
                        );
                        Err(ShellError::OutsideSpannedLabeledError {
                            src,
                            error: "day duration too large".into(),
                            msg: "day duration too large".into(),
                            span: label_span,
                        })
                    }
                },

                Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => {
                        let (src, label_span) = truncated_source_window(
                            original_text,
                            expr.span,
                            DEFAULT_ERROR_CONTEXT,
                        );
                        Err(ShellError::OutsideSpannedLabeledError {
                            src,
                            error: "week duration too large".into(),
                            msg: "week duration too large".into(),
                            span: label_span,
                        })
                    }
                },
            }
        }
        Expr::Var(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "variables not supported in nuon",
        )),
        Expr::VarDecl(..) => Err(truncated_nuon_error(
            original_text,
            expr.span,
            "variable declarations not supported in nuon",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nuon_parse_success() {
        let result = from_nuon("{ a: 1, b: [2, 3] }", Some(Span::test_data()));
        assert!(result.is_ok(), "valid NUON should parse");
    }

    // Bug 8 in crates/nuon/spec/bugs_to_fix.md. The spec
    // (crates/nuon/spec/nuon_formal_specification.md, "durations and filesizes")
    // says: "overflow is an **error**; it must not wrap or saturate. a positive
    // literal must never read back as a negative duration."
    //
    // The parser clamps an out-of-range head to i64::MAX and keeps the unit it
    // folded to, so `[1e30sec]` reaches this function as Int(i64::MAX) plus
    // Unit::Millisecond. `size * 1000 * 1000` then overflowed: a panic in debug
    // builds and -1000000ns in release ones. Unit::Day and Unit::Week already
    // used checked_mul; the units that real `ms`/`sec`/`min`/`hr`/`day` literals
    // fold to did not.
    //
    // `ns` and `us` fold to Unit::Nanosecond, which multiplies by nothing, so
    // they are not covered here.
    #[test]
    fn duration_overflow_is_rejected() {
        for unit in ["ms", "sec", "min", "hr", "day", "wk"] {
            let input = format!("[1e30{unit}]");
            let result = from_nuon(&input, Some(Span::test_data()));
            assert!(
                result.is_err(),
                "`{input}` should be rejected as an overflow, got {result:?}"
            );
        }
    }

    #[test]
    fn in_range_durations_still_decode() {
        // 1000000000ns is 1 second, comfortably inside the 64-bit range.
        let value = from_nuon("[1000000000ns]", Some(Span::test_data()))
            .expect("1000000000ns is in range and should decode");
        assert!(matches!(
            value,
            Value::List { vals, .. }
                if matches!(vals.first(), Some(Value::Duration { val, .. }) if *val == 1_000_000_000)
        ));
    }

    #[test]
    fn nuon_error_source_is_bounded() {
        // Build a large valid NUON with a parse error
        let mut input = String::with_capacity(50_000);
        input.push_str("{ ");
        for i in 0..2000 {
            use std::fmt::Write;
            write!(&mut input, "key{i}: \"value{i}\", ").unwrap();
        }
        // Syntax error: unclosed string
        input.push_str("key_last: \"unclosed ");
        input.push_str(" }");

        let result = from_nuon(&input, Some(Span::test_data()));
        assert!(result.is_err(), "should fail to parse");

        // The error should not contain the full 50KB source
        let err_str = format!("{err:?}", err = result.as_ref().unwrap_err());
        assert!(
            err_str.len() < 25_000,
            "error debug output should be bounded, got {} bytes",
            err_str.len()
        );
    }

    #[test]
    fn nuon_error_for_invalid_syntax() {
        let result = from_nuon("{ a: 1, b: ]", Some(Span::test_data()));
        assert!(result.is_err(), "invalid NUON should error");
    }
}
