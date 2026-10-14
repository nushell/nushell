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

    // Strip exactly one leading byte order mark. Windows editors and PowerShell's
    // `Out-File` write one, so a reader that refuses them cannot open files people
    // actually have. `U+FEFF` anywhere else is ordinary content, so this is a
    // `strip_prefix` rather than a `trim`.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);

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

                // Every duration unit goes through `Unit::build_value`, which
                // already checks the multiplication against the 64-bit range.
                // A plain `size * factor` would panic in debug builds and wrap
                // to a negative duration in release ones, and the nuon spec
                // requires an error. The unit is not named in the message: the
                // parser has already folded the one that was written, so
                // `1e30sec` arrives as a millisecond count.
                unit => unit.build_value(size, span).map_err(|_| {
                    truncated_nuon_error(original_text, expr.span, "duration too large")
                }),
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
    use nu_protocol::{Spanned, ast::ValueWithUnit};

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
    //
    // A negative head is specified to be allowed (the spec shows `-1_000ns` as
    // valid), so `[-1sec]` has to keep decoding — but the clamped negative head
    // is the same i64::MIN the positive one is, and the raw multiplies it used
    // to reach turned it into exactly 0ns, which is neither the number written
    // nor an error. Both signs go through the same arms, so both are pinned.
    #[test]
    fn duration_overflow_is_rejected() {
        for unit in ["ms", "sec", "min", "hr", "day", "wk"] {
            for sign in ["", "-"] {
                let input = format!("[{sign}1e30{unit}]");
                let err = from_nuon(&input, Some(Span::test_data()))
                    .expect_err("an overflowing duration must not decode");
                let (error, msg) = match err {
                    ShellError::OutsideSpannedLabeledError { error, msg, .. } => (error, msg),
                    other => {
                        panic!("`{input}` should report a labeled duration overflow, got {other:?}")
                    }
                };
                // The unit is deliberately not named: the parser folds the one that
                // was written, so `1e30sec` reaches the reader as a millisecond
                // count and `1e30wk` as a day count.
                assert_eq!(
                    msg, "duration too large",
                    "unexpected message for `{input}`"
                );
                assert_eq!(
                    error, "Error when loading",
                    "unexpected title for `{input}`"
                );
                assert!(
                    !msg.contains("millisecond") && !msg.contains("day"),
                    "`{input}` should not name a folded unit, got {msg}"
                );
            }
        }
    }

    #[test]
    fn duration_overflow_error_points_at_the_literal() {
        // The label has to cover the literal that overflowed, not whatever span
        // the caller handed in. A short input makes the two indistinguishable in
        // the rendered text, so assert on the label itself.
        let input = "[1e30sec]";
        let err = from_nuon(input, Some(Span::new(0, 4096)))
            .expect_err("an overflowing duration must not decode");
        let ShellError::OutsideSpannedLabeledError { src, span, .. } = &err else {
            panic!("expected a labeled span error, got {err:?}");
        };
        assert_eq!(
            &src[span.start..span.end],
            "1e30sec",
            "the label should cover the literal that overflowed"
        );
    }

    #[test]
    fn in_range_durations_still_decode() {
        // The parser folds valid durations into nanoseconds. Keep this check on
        // the public NUON path so that the parser-to-reader behavior is covered.
        for (literal, expected_ns) in [
            ("1ns", 1_i64),
            ("1us", 1_000),
            ("1ms", 1_000_000),
            ("1sec", 1_000_000_000),
            ("1min", 60_000_000_000),
            ("1hr", 3_600_000_000_000),
            ("1day", 86_400_000_000_000),
            ("1wk", 604_800_000_000_000),
        ] {
            let input = format!("[{literal}]");
            let value = from_nuon(&input, Some(Span::test_data()))
                .unwrap_or_else(|err| panic!("`{literal}` is in range and should decode: {err:?}"));
            let Value::List { vals, .. } = value else {
                panic!("`{literal}` should decode to a list");
            };
            let Some(Value::Duration { val, .. }) = vals.first() else {
                panic!("`{literal}` should decode to a duration");
            };
            assert_eq!(*val, expected_ns, "unexpected duration for `{literal}`");
        }
    }

    #[test]
    fn scaled_duration_units_still_decode() {
        // Valid source literals are folded to nanoseconds by the parser, so build
        // the AST directly to exercise Unit::build_value's scaled success paths.
        for (unit, expected_ns) in [
            (Unit::Microsecond, 1_000_i64),
            (Unit::Millisecond, 1_000_000),
            (Unit::Second, 1_000_000_000),
            (Unit::Minute, 60_000_000_000),
            (Unit::Hour, 3_600_000_000_000),
            (Unit::Day, 86_400_000_000_000),
            (Unit::Week, 604_800_000_000_000),
        ] {
            let literal = "1 unit";
            let literal_span = Span::new(0, literal.len());
            let value = ValueWithUnit {
                expr: Expression::new_unknown(Expr::Int(1), Span::new(0, 1), Type::Number),
                unit: Spanned {
                    item: unit,
                    span: Span::new(2, literal.len()),
                },
            };
            let expr = Expression::new_unknown(
                Expr::ValueWithUnit(Box::new(value)),
                literal_span,
                Type::Duration,
            );
            let actual = convert_to_value(expr, Span::test_data(), literal)
                .unwrap_or_else(|err| panic!("{unit:?} should decode in range: {err:?}"));
            assert!(
                matches!(actual, Value::Duration { val, .. } if val == expected_ns),
                "unexpected duration for {unit:?}: {actual:?}"
            );
        }
    }

    #[test]
    fn nuon_parse_strips_one_leading_bom() {
        // Windows editors and PowerShell's `Out-File` write a bom; a reader that
        // rejects one cannot open files people actually have.
        for input in ["\u{feff}1", "\u{feff}[1]", "\u{feff}{ a: 1 }"] {
            let with_bom = from_nuon(input, Some(Span::test_data()));
            let without_bom = from_nuon(
                input.trim_start_matches('\u{feff}'),
                Some(Span::test_data()),
            );
            assert!(
                with_bom.is_ok(),
                "a leading bom should be stripped, but {input:?} failed: {with_bom:?}"
            );
            assert_eq!(
                with_bom.expect("should parse"),
                without_bom.expect("should parse"),
                "a leading bom should not change the decoded value of {input:?}"
            );
        }
    }

    #[test]
    fn nuon_parse_keeps_a_lone_bom_and_inner_boms() {
        // Exactly one *leading* bom is stripped, and U+FEFF anywhere else is ordinary
        // content. A document that is only a bom is an empty document, which is bug 2
        // in the spec's list and deliberately not addressed here.

        // A bom inside a string body is content, not a prefix.
        let inner = from_nuon("[\"\u{feff}\"]", Some(Span::test_data())).expect("should parse");
        assert_eq!(
            inner,
            Value::list(vec![Value::test_string("\u{feff}")], Span::test_data()),
            "an inner bom is content, not a prefix"
        );

        // Two leading boms: only the first is stripped, so the second becomes a bare
        // word that fails to parse rather than being silently dropped as well.
        assert!(
            from_nuon("\u{feff}\u{feff}1", Some(Span::test_data())).is_err(),
            "only one leading bom is stripped"
        );
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
