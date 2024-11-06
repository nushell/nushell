use nu_protocol::{
    ast::{Expr, Expression, ListItem, RecordItem},
    engine::{EngineState, StateWorkingSet},
    Filesize, IntoValue, Range, Record, ShellError, Span, Type, Unit, Value,
};
use std::sync::Arc;

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
            return Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span,
                help: None,
                inner: vec![ShellError::OutsideSpannedLabeledError {
                    src: input.to_string(),
                    error: "error when loading".into(),
                    msg: "excess values when loading".into(),
                    span: element.expr.span,
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
            span.unwrap_or(Span::unknown()),
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
                    span: expr.expr.span,
                }],
            });
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

    let value = convert_to_value(expr, span.unwrap_or(Span::unknown()), input)?;

    Ok(value)
}

fn convert_to_value(
    expr: Expression,
    span: Span,
    original_text: &str,
) -> Result<Value, ShellError> {
    match expr.expr {
        Expr::BinaryOp(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "binary operators not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::UnaryNot(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "unary operators not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Block(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "blocks not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Closure(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "closures not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Binary(val) => Ok(Value::binary(val, span)),
        Expr::Bool(val) => Ok(Value::bool(val, span)),
        Expr::Call(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "calls not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::CellPath(val) => Ok(Value::cell_path(val, span)),
        Expr::DateTime(dt) => Ok(Value::date(dt, span)),
        Expr::ExternalCall(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "calls not supported in nuon".into(),
            span: expr.span,
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
                    span: expr.span,
                })
            } else {
                convert_to_value(full_cell_path.head, span, original_text)
            }
        }

        Expr::Garbage => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "extra tokens in input file".into(),
            span: expr.span,
        }),
        Expr::GlobPattern(val, _) => Ok(Value::string(val, span)),
        Expr::ImportPattern(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "imports not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Overlay(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "overlays not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Int(val) => Ok(Value::int(val, span)),
        Expr::Keyword(kw) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: format!(
                "{} not supported in nuon",
                String::from_utf8_lossy(&kw.keyword)
            ),
            span: expr.span,
        }),
        Expr::List(vals) => {
            let mut output = vec![];

            for item in vals {
                match item {
                    ListItem::Item(expr) => {
                        output.push(convert_to_value(expr, span, original_text)?);
                    }
                    ListItem::Spread(_, inner) => {
                        return Err(ShellError::OutsideSpannedLabeledError {
                            src: original_text.to_string(),
                            error: "Error when loading".into(),
                            msg: "spread operator not supported in nuon".into(),
                            span: inner.span,
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
            span: expr.span,
        }),
        Expr::Nothing => Ok(Value::nothing(span)),
        Expr::Operator(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "operators not supported in nuon".into(),
            span: expr.span,
        }),
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
                                return Err(ShellError::OutsideSpannedLabeledError {
                                    src: original_text.to_string(),
                                    error: "Error when loading".into(),
                                    msg: "only strings can be keys".into(),
                                    span: key.span,
                                })
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
                        return Err(ShellError::OutsideSpannedLabeledError {
                            src: original_text.to_string(),
                            error: "Error when loading".into(),
                            msg: "spread operator not supported in nuon".into(),
                            span: inner.span,
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
            span: expr.span,
        }),
        Expr::Signature(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "signatures not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::String(s) | Expr::RawString(s) => Ok(Value::string(s, span)),
        Expr::StringInterpolation(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "string interpolation not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::GlobInterpolation(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "glob interpolation not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Collect(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "`$in` not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Subexpression(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "subexpressions not supported in nuon".into(),
            span: expr.span,
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
                            span: expr.span,
                        })
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
                    return Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "Error when loading".into(),
                        msg: "table has mismatched columns".into(),
                        span: expr.span,
                    });
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
                    return Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "Error when loading".into(),
                        msg: "non-integer unit value".into(),
                        span: expr.span,
                    })
                }
            };

            match value.unit.item {
                Unit::Filesize(unit) => match Filesize::from_unit(size, unit) {
                    Some(val) => Ok(val.into_value(span)),
                    None => Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.into(),
                        error: "filesize too large".into(),
                        msg: "filesize too large".into(),
                        span: expr.span,
                    }),
                },

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
                        span: expr.span,
                    }),
                },

                Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                    Some(val) => Ok(Value::duration(val, span)),
                    None => Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "week duration too large".into(),
                        msg: "week duration too large".into(),
                        span: expr.span,
                    }),
                },
            }
        }
        Expr::Var(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "variables not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::VarDecl(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "variable declarations not supported in nuon".into(),
            span: expr.span,
        }),
    }
}
