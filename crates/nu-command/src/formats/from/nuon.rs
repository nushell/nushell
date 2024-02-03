use nu_protocol::ast::{Call, Expr, Expression, PipelineElement, RecordItem};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    record, Category, Example, IntoPipelineData, PipelineData, Range, Record, ShellError,
    Signature, Span, Type, Unit, Value,
};
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
        let (string_input, _span, metadata) = input.collect_string_strict(head)?;

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
                        span: element.span(),
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

        let expr = if block.pipelines.is_empty() {
            Expression {
                expr: Expr::Nothing,
                span: head,
                custom_completion: None,
                ty: Type::Nothing,
            }
        } else {
            let mut pipeline = block.pipelines.remove(0);

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
                        span: expr.span(),
                    }],
                });
            }

            if pipeline.elements.is_empty() {
                Expression {
                    expr: Expr::Nothing,
                    span: head,
                    custom_completion: None,
                    ty: Type::Nothing,
                }
            } else {
                match pipeline.elements.remove(0) {
                    PipelineElement::Expression(_, expression)
                    | PipelineElement::Redirection(_, _, expression, _)
                    | PipelineElement::And(_, expression)
                    | PipelineElement::Or(_, expression)
                    | PipelineElement::SameTargetRedirection {
                        cmd: (_, expression),
                        ..
                    }
                    | PipelineElement::SeparateRedirection {
                        out: (_, expression, _),
                        ..
                    } => expression,
                }
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

        let result = convert_to_value(expr, head, &string_input);

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
        Expr::CellPath(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "subexpressions and cellpaths not supported in nuon".into(),
            span: expr.span,
        }),
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
        Expr::Keyword(kw, ..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: format!("{} not supported in nuon", String::from_utf8_lossy(&kw)),
            span: expr.span,
        }),
        Expr::List(vals) => {
            let mut output = vec![];
            for val in vals {
                output.push(convert_to_value(val, span, original_text)?);
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
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                convert_to_value(*f, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            let next = if let Some(s) = next {
                convert_to_value(*s, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            let to = if let Some(t) = to {
                convert_to_value(*t, span, original_text)?
            } else {
                Value::nothing(expr.span)
            };

            Ok(Value::range(
                Range::new(expr.span, from, next, to, &operator)?,
                expr.span,
            ))
        }
        Expr::Record(key_vals) => {
            let mut record = Record::new();

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

                        let value = convert_to_value(val, span, original_text)?;

                        record.push(key_str, value);
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
        Expr::Spread(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "spread operator not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::String(s) => Ok(Value::string(s, span)),
        Expr::StringInterpolation(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "string interpolation not supported in nuon".into(),
            span: expr.span,
        }),
        Expr::Subexpression(..) => Err(ShellError::OutsideSpannedLabeledError {
            src: original_text.to_string(),
            error: "Error when loading".into(),
            msg: "subexpressions not supported in nuon".into(),
            span: expr.span,
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
                            span: expr.span,
                        })
                    }
                };

                if let Some(idx) = cols.iter().position(|existing| existing == key_str) {
                    return Err(ShellError::ColumnDefinedTwice {
                        col_name: key_str.clone(),
                        second_use: key.span,
                        first_use: headers[idx].span,
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
                        span: expr.span,
                    });
                }
                let vals: Vec<Value> = row
                    .into_iter()
                    .map(|cell| convert_to_value(cell, span, original_text))
                    .collect::<Result<_, _>>()?;

                output.push(Value::record(
                    Record::from_raw_cols_vals_unchecked(cols.clone(), vals),
                    span,
                ));
            }

            Ok(Value::list(output, span))
        }
        Expr::ValueWithUnit(val, unit) => {
            let size = match val.expr {
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

            match unit.item {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(FromNuon {})
    }
}
