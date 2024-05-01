use nu_protocol::{
    ast::{Expr, Expression, ListItem, RecordItem},
    engine::{EngineState, StateWorkingSet},
    FutureSpanId, Range, Record, ShellError, Type, Unit, Value,
};
use std::sync::Arc;

/// convert a raw string representation of NUON data to an actual Nushell [`Value`]
///
/// > **Note**
/// > [`FutureSpanId`] can be passed to [`from_nuon`] if there is context available to the caller, e.g. when
/// > using this function in a command implementation such as
/// [`from nuon`](https://www.nushell.sh/commands/docs/from_nuon.html).
///
/// also see [`super::to_nuon`] for the inverse operation
pub fn from_nuon(input: &str, span: Option<FutureSpanId>) -> Result<Value, ShellError> {
    let mut engine_state = EngineState::default();
    // NOTE: the parser needs `$env.PWD` to be set, that's a know _API issue_ with the
    // [`EngineState`]
    engine_state.add_env_var(
        "PWD".to_string(),
        Value::string("", FutureSpanId::unknown()),
    );
    let mut working_set = StateWorkingSet::new(&engine_state);

    let mut block = nu_parser::parse(&mut working_set, None, input.as_bytes(), false);

    if let Some(pipeline) = block.pipelines.get(1) {
        if let Some(element) = pipeline.elements.first() {
            return Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span,
                help: None,
                inner: vec![ShellError::OutsideSpannedLabeledError {
                    src: input.to_string(),
                    error: "error when loading".into(),
                    msg: "excess values when loading".into(),
                    span: element.expr.get_span(&working_set),
                }],
            });
        } else {
            return Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span,
                help: None,
                inner: vec![ShellError::GenericError {
                    error: "error when loading".into(),
                    msg: "excess values when loading".into(),
                    span,
                    help: None,
                    inner: vec![],
                }],
            });
        }
    }

    let expr = if block.pipelines.is_empty() {
        Expression::new(
            &mut working_set,
            Expr::Nothing,
            span.unwrap_or(FutureSpanId::unknown()).span(),
            Type::Nothing,
        )
    } else {
        let mut pipeline = Arc::make_mut(&mut block).pipelines.remove(0);

        if let Some(expr) = pipeline.elements.get(1) {
            return Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span,
                help: None,
                inner: vec![ShellError::OutsideSpannedLabeledError {
                    src: input.to_string(),
                    error: "error when loading".into(),
                    msg: "detected a pipeline in nuon file".into(),
                    span: expr.expr.get_span(&working_set),
                }],
            });
        }

        if pipeline.elements.is_empty() {
            Expression::new(
                &mut working_set,
                Expr::Nothing,
                span.unwrap_or(FutureSpanId::unknown()).span(),
                Type::Nothing,
            )
        } else {
            pipeline.elements.remove(0).expr
        }
    };

    if let Some(err) = working_set.parse_errors.first() {
        return Err(ShellError::GenericError {
            error: "error when parsing nuon text".into(),
            msg: "could not parse nuon text".into(),
            span,
            help: None,
            inner: vec![ShellError::OutsideSpannedLabeledError {
                src: input.to_string(),
                error: "error when parsing".into(),
                msg: err.to_string(),
                span: err.span(),
            }],
        });
    }

    let value = convert_to_value(
        &working_set,
        expr,
        span.unwrap_or(FutureSpanId::unknown()),
        input,
    )?;

    Ok(value)
}

fn convert_to_value(
    working_set: &StateWorkingSet,
    expr: Expression,
    span: FutureSpanId,
    original_text: &str,
) -> Result<Value, ShellError> {
    let expr_span = expr.get_span(working_set);

    match expr.expr {
        Expr::BinaryOp(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "binary operators not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::UnaryNot(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "unary operators not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Block(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "blocks not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Closure(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "closures not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Binary(val) => Ok(Value::binary(val, span)),
        Expr::Bool(val) => Ok(Value::bool(val, span)),
        Expr::Call(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "calls not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::CellPath(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "subexpressions and cellpaths not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::DateTime(dt) => Ok(Value::date(dt, span)),
        Expr::ExternalCall(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "calls not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Filepath(val, _) => Ok(Value::string(val, span)),
        Expr::Directory(val, _) => Ok(Value::string(val, span)),
        Expr::Float(val) => Ok(Value::float(val, span)),
        Expr::FullCellPath(full_cell_path) => {
            if !full_cell_path.tail.is_empty() {
                Err(ShellError::OutsideSpannedLabeledError {
                    src: original_text.to_string(),
                    error: "Error when loading".into(),
                    msg: "subexpressions and cellpaths not supported in nuon".into(),
                    span: expr_span,
                })
            } else {
                convert_to_value(working_set, full_cell_path.head, span, original_text)
            }
        }

        Expr::Garbage => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "extra tokens in input file".into(),
            span: expr_span,
        }),
        Expr::GlobPattern(val, _) => Ok(Value::string(val, span)),
        Expr::ImportPattern(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "imports not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Overlay(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "overlays not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Int(val) => Ok(Value::int(val, span)),
        Expr::Keyword(kw) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: format!(
                "{} not supported in nuon",
                String::from_utf8_lossy(&kw.keyword)
            ),
            span: expr_span,
        }),
        Expr::List(vals) => {
            let mut output = vec![];

            for item in vals {
                match item {
                    ListItem::Item(expr) => {
                        output.push(convert_to_value(working_set, expr, span, original_text)?);
                    }
                    ListItem::Spread(_, inner) => {
                        return Err(ShellError::OutsideSpannedLabeledError {
                            src: original_text.to_string(),
                            error: "Error when loading".into(),
                            msg: "spread operator not supported in nuon".into(),
                            span: inner.get_span(working_set),
                        });
                    }
                }
            }

            Ok(Value::list(output, span))
        }
        Expr::MatchBlock(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "match blocks not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Nothing => Ok(Value::nothing(span)),
        Expr::Operator(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "operators not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Range(range) => {
            let from = if let Some(f) = range.from {
                convert_to_value(working_set, f, span, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            let next = if let Some(s) = range.next {
                convert_to_value(working_set, s, span, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            let to = if let Some(t) = range.to {
                convert_to_value(working_set, t, span, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            Ok(Value::range(
                Range::new(from, next, to, range.operator.inclusion, expr_span)?,
                expr_span,
            ))
        }
        Expr::Record(key_vals) => {
            let mut record = Record::with_capacity(key_vals.len());
            let mut key_spans = Vec::with_capacity(key_vals.len());

            for key_val in key_vals {
                match key_val {
                    RecordItem::Pair(key, val) => {
                        let key_span = key.get_span(working_set);

                        let key_str = match key.expr {
                            Expr::String(key_str) => key_str,
                            _ => {
                                return Err(ShellError::OutsideSpannedLabeledError {
                                    src: original_text.to_string(),
                                    error: "Error when loading".into(),
                                    msg: "only strings can be keys".into(),
                                    span: key.get_span(working_set),
                                })
                            }
                        };

                        if let Some(i) = record.index_of(&key_str) {
                            return Err(ShellError::ColumnDefinedTwice {
                                col_name: key_str,
                                second_use: key_span,
                                first_use: key_spans[i],
                            });
                        } else {
                            key_spans.push(key_span);
                            record.push(
                                key_str,
                                convert_to_value(working_set, val, span, original_text)?,
                            );
                        }
                    }
                    RecordItem::Spread(_, inner) => {
                        return Err(ShellError::OutsideSpannedLabeledError {
                            src: original_text.to_string(),
                            error: "Error when loading".into(),
                            msg: "spread operator not supported in nuon".into(),
                            span: inner.get_span(working_set),
                        });
                    }
                }
            }

            Ok(Value::record(record, span))
        }
        Expr::RowCondition(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "row conditions not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Signature(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "signatures not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::String(s) => Ok(Value::string(s, span)),
        Expr::StringInterpolation(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "string interpolation not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Subexpression(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "subexpressions not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Table(mut table) => {
            let mut cols = vec![];

            let mut output = vec![];

            for key in table.columns.as_mut() {
                let key_str = match &mut key.expr {
                    Expr::String(key_str) => key_str,
                    _ => {
                        return Err(ShellError::OutsideSpannedLabeledError {
                            src: original_text.to_string(),
                            error: "Error when loading".into(),
                            msg: "only strings can be keys".into(),
                            span: expr_span,
                        })
                    }
                };

                if let Some(idx) = cols.iter().position(|existing| existing == key_str) {
                    return Err(ShellError::ColumnDefinedTwice {
                        col_name: key_str.clone(),
                        second_use: key.get_span(working_set),
                        first_use: table.columns[idx].get_span(working_set),
                    });
                } else {
                    cols.push(std::mem::take(key_str));
                }
            }

            for row in table.rows.into_vec() {
                if cols.len() != row.len() {
                    return Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "Error when loading".into(),
                        msg: "table has mismatched columns".into(),
                        span: expr_span,
                    });
                }

                let record = cols
                    .iter()
                    .zip(row.into_vec())
                    .map(|(col, cell)| {
                        convert_to_value(working_set, cell, span, original_text)
                            .map(|val| (col.clone(), val))
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
                    return Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "Error when loading".into(),
                        msg: "non-integer unit value".into(),
                        span: expr_span,
                    })
                }
            };

            match value.unit.item {
                Unit::Byte => Ok(Value::filesize(size, span)),
                Unit::Kilobyte => Ok(Value::filesize(size * 1000, span)),
                Unit::Megabyte => Ok(Value::filesize(size * 1000 * 1000, span)),
                Unit::Gigabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000, span)),
                Unit::Terabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000 * 1000, span)),
                Unit::Petabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                )),
                Unit::Exabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                )),

                Unit::Kibibyte => Ok(Value::filesize(size * 1024, span)),
                Unit::Mebibyte => Ok(Value::filesize(size * 1024 * 1024, span)),
                Unit::Gibibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024, span)),
                Unit::Tebibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024 * 1024, span)),
                Unit::Pebibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                )),
                Unit::Exbibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                )),

                Unit::Nanosecond => Ok(Value::duration(size, span)),
                Unit::Microsecond => Ok(Value::duration(size * 1000, span)),
                Unit::Millisecond => Ok(Value::duration(size * 1000 * 1000, span)),
                Unit::Second => Ok(Value::duration(size * 1000 * 1000 * 1000, span)),
                Unit::Minute => Ok(Value::duration(size * 1000 * 1000 * 1000 * 60, span)),
                Unit::Hour => Ok(Value::duration(size * 1000 * 1000 * 1000 * 60 * 60, span)),
                Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "day duration too large".into(),
                        msg: "day duration too large".into(),
                        span: expr_span,
                    }),
                },

                Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "week duration too large".into(),
                        msg: "week duration too large".into(),
                        span: expr_span,
                    }),
                },
            }
        }
        Expr::Var(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "variables not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::VarDecl(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "variable declarations not supported in nuon".into(),
            span: expr_span,
        }),
    }
}
