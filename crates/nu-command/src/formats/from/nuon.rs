use nu_engine::command_prelude::*;
use nu_protocol::{ast::{Expr, Expression, RecordItem}, engine::{StateWorkingSet, UNKNOWN_SPAN_ID}, Range, SpanId, Unit};
use std::sync::Arc;

#[derive(Clone)]
pub struct FromNuon;

impl Command for FromNuon {
    fn name(&self) -> &str {
        "from nuon"
    }

    fn usage(&self) -> &str {
        "Convert from nuon to structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from nuon")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'{ a:1 }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
            Example {
                example: "'{ a:1, b: [1, 2] }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                    "b" => Value::test_list(vec![Value::test_int(1), Value::test_int(2)]),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let head_id = call.head_id;
        let (string_input, _span, _span_id, metadata) = input.collect_string_strict(head, head_id)?;

        let engine_state = engine_state.clone();

        let mut working_set = StateWorkingSet::new(&engine_state);

        let mut block = nu_parser::parse(&mut working_set, None, string_input.as_bytes(), false);

        if let Some(pipeline) = block.pipelines.get(1) {
            if let Some(element) = pipeline.elements.first() {
                return Err(ShellError::GenericError {
                    error: "error when loading nuon text".into(),
                    msg: "could not load nuon text".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![ShellError::OutsideSpannedLabeledError {
                        src: string_input,
                        error: "error when loading".into(),
                        msg: "excess values when loading".into(),
                        span: element.expr.get_span(&engine_state),
                    }],
                });
            } else {
                return Err(ShellError::GenericError {
                    error: "error when loading nuon text".into(),
                    msg: "could not load nuon text".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![ShellError::GenericError {
                        error: "error when loading".into(),
                        msg: "excess values when loading".into(),
                        span: Some(head),
                        help: None,
                        inner: vec![],
                    }],
                });
            }
        }

        let head_id = engine_state.find_span_id(head).unwrap_or(UNKNOWN_SPAN_ID);

        let expr = if block.pipelines.is_empty() {
            Expression::new_existing(Expr::Nothing, head_id, Type::Nothing)
        } else {
            let mut pipeline = Arc::make_mut(&mut block).pipelines.remove(0);

            if let Some(expr) = pipeline.elements.get(1) {
                return Err(ShellError::GenericError {
                    error: "error when loading nuon text".into(),
                    msg: "could not load nuon text".into(),
                    span: Some(head),
                    help: None,
                    inner: vec![ShellError::OutsideSpannedLabeledError {
                        src: string_input,
                        error: "error when loading".into(),
                        msg: "detected a pipeline in nuon file".into(),
                        span: expr.expr.get_span(&engine_state),
                    }],
                });
            }

            if pipeline.elements.is_empty() {
                Expression::new_existing(Expr::Nothing, head_id, Type::Nothing)
            } else {
                pipeline.elements.remove(0).expr
            }
        };

        if let Some(err) = working_set.parse_errors.first() {
            return Err(ShellError::GenericError {
                error: "error when parsing nuon text".into(),
                msg: "could not parse nuon text".into(),
                span: Some(head),
                help: None,
                inner: vec![ShellError::OutsideSpannedLabeledError {
                    src: string_input,
                    error: "error when parsing".into(),
                    msg: err.to_string(),
                    span: err.span(),
                }],
            });
        }

        let result = convert_to_value(&working_set, expr, head, head_id, &string_input);

        match result {
            Ok(result) => Ok(result.into_pipeline_data_with_metadata(metadata)),
            Err(err) => Err(ShellError::GenericError {
                error: "error when loading nuon text".into(),
                msg: "could not load nuon text".into(),
                span: Some(head),
                help: None,
                inner: vec![err],
            }),
        }
    }
}

fn convert_to_value(
    working_set: &StateWorkingSet,
    expr: Expression,
    head_span: Span,
    head_span_id: SpanId,
    original_text: &str,
) -> Result<Value, ShellError> {
    let expr_span_id = expr.span_id;
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
        Expr::Binary(val) => Ok(Value::binary(val, head_span)),
        Expr::Bool(val) => Ok(Value::bool(val, head_span_id)),
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
        Expr::DateTime(dt) => Ok(Value::date(dt, head_span)),
        Expr::ExternalCall(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "calls not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Filepath(val, _) => Ok(Value::string(val, head_span)),
        Expr::Directory(val, _) => Ok(Value::string(val, head_span)),
        Expr::Float(val) => Ok(Value::float(val, head_span)),
        Expr::FullCellPath(full_cell_path) => {
            if !full_cell_path.tail.is_empty() {
                Err(ShellError::OutsideSpannedLabeledError {
                    src: original_text.to_string(),
                    error: "Error when loading".into(),
                    msg: "subexpressions and cellpaths not supported in nuon".into(),
                    span: expr_span,
                })
            } else {
                convert_to_value(working_set, full_cell_path.head, head_span, head_span_id, original_text)
            }
        }

        Expr::Garbage => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "extra tokens in input file".into(),
            span: expr_span,
        }),
        Expr::GlobPattern(val, _) => Ok(Value::string(val, head_span)),
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
        Expr::Int(val) => Ok(Value::int(val, head_span)),
        Expr::Keyword(kw, ..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: format!("{} not supported in nuon", String::from_utf8_lossy(&kw)),
            span: expr_span,
        }),
        Expr::List(vals) => {
            let mut output = vec![];
            for val in vals {
                output.push(convert_to_value(
                    working_set,
                    val,
                    head_span,
                    head_span_id,
                    original_text,
                )?);
            }

            Ok(Value::list(output, head_span))
        }
        Expr::MatchBlock(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "match blocks not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Nothing => Ok(Value::nothing(head_span)),
        Expr::Operator(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "operators not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                convert_to_value(working_set, *f, head_span, head_span_id, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            let next = if let Some(s) = next {
                convert_to_value(working_set, *s, head_span, head_span_id, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            let to = if let Some(t) = to {
                convert_to_value(working_set, *t, head_span, head_span_id, original_text)?
            } else {
                Value::nothing(expr_span)
            };

            Ok(Value::range(
                Range::new(expr_span, expr_span_id, from, next, to, &operator)?,
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
                                    span: key_span,
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
                                convert_to_value(working_set, val, head_span, head_span_id, original_text)?,
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

            Ok(Value::record(record, head_span))
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
        Expr::Spread(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "spread operator not supported in nuon".into(),
            span: expr_span,
        }),
        Expr::String(s) => Ok(Value::string(s, head_span)),
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
        Expr::Table(mut headers, cells) => {
            let mut cols = vec![];

            let mut output = vec![];

            for key in headers.iter_mut() {
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
                        first_use: headers[idx].get_span(working_set),
                    });
                } else {
                    cols.push(std::mem::take(key_str));
                }
            }

            for row in cells {
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
                    .zip(row)
                    .map(|(col, cell)| {
                        convert_to_value(working_set, cell, head_span, head_span_id, original_text)
                            .map(|val| (col.clone(), val))
                    })
                    .collect::<Result<_, _>>()?;

                output.push(Value::record(record, head_span));
            }

            Ok(Value::list(output, head_span))
        }
        Expr::ValueWithUnit(val, unit) => {
            let size = match val.expr {
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

            match unit.item {
                Unit::Byte => Ok(Value::filesize(size, head_span)),
                Unit::Kilobyte => Ok(Value::filesize(size * 1000, head_span)),
                Unit::Megabyte => Ok(Value::filesize(size * 1000 * 1000, head_span)),
                Unit::Gigabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000, head_span)),
                Unit::Terabyte => Ok(Value::filesize(size * 1000 * 1000 * 1000 * 1000, head_span)),
                Unit::Petabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000,
                    head_span,
                )),
                Unit::Exabyte => Ok(Value::filesize(
                    size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                    head_span,
                )),

                Unit::Kibibyte => Ok(Value::filesize(size * 1024, head_span)),
                Unit::Mebibyte => Ok(Value::filesize(size * 1024 * 1024, head_span)),
                Unit::Gibibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024, head_span)),
                Unit::Tebibyte => Ok(Value::filesize(size * 1024 * 1024 * 1024 * 1024, head_span)),
                Unit::Pebibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024,
                    head_span,
                )),
                Unit::Exbibyte => Ok(Value::filesize(
                    size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                    head_span,
                )),

                Unit::Nanosecond => Ok(Value::duration(size, head_span)),
                Unit::Microsecond => Ok(Value::duration(size * 1000, head_span)),
                Unit::Millisecond => Ok(Value::duration(size * 1000 * 1000, head_span)),
                Unit::Second => Ok(Value::duration(size * 1000 * 1000 * 1000, head_span)),
                Unit::Minute => Ok(Value::duration(size * 1000 * 1000 * 1000 * 60, head_span)),
                Unit::Hour => Ok(Value::duration(
                    size * 1000 * 1000 * 1000 * 60 * 60,
                    head_span,
                )),
                Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                    Some(val) => Ok(Value::duration(val, head_span)),
                    None => Err(ShellError::OutsideSpannedLabeledError {
                        src: original_text.to_string(),
                        error: "day duration too large".into(),
                        msg: "day duration too large".into(),
                        span: expr_span,
                    }),
                },

                Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                    Some(val) => Ok(Value::duration(val, head_span)),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromNuon {})
    }
}
