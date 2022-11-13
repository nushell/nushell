use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Range, ShellError, Signature, Span, Type,
    Unit, Value,
};
#[derive(Clone)]
pub struct FromNuon;

impl Command for FromNuon {
    fn name(&self) -> &str {
        "from nuon"
    }

    fn usage(&self) -> &str {
        "Convert from nuon to structured data"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from nuon")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Experimental)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "'{ a:1 }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string()],
                    vals: vec![Value::Int {
                        val: 1,
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                example: "'{ a:1, b: [1, 2] }' | from nuon",
                description: "Converts nuon formatted string to table",
                result: Some(Value::Record {
                    cols: vec!["a".to_string(), "b".to_string()],
                    vals: vec![
                        Value::Int {
                            val: 1,
                            span: Span::test_data(),
                        },
                        Value::List {
                            vals: vec![
                                Value::Int {
                                    val: 1,
                                    span: Span::test_data(),
                                },
                                Value::Int {
                                    val: 2,
                                    span: Span::test_data(),
                                },
                            ],
                            span: Span::test_data(),
                        },
                    ],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        let head = call.head;
        let config = engine_state.get_config();
        let string_input = input.collect_string("", config)?;

        let engine_state = EngineState::new();

        let mut working_set = StateWorkingSet::new(&engine_state);

        let _ = working_set.add_file("nuon file".to_string(), string_input.as_bytes());

        let mut error = None;

        let (lexed, err) = nu_parser::lex(string_input.as_bytes(), 0, &[b'\n', b'\r'], &[], true);
        error = error.or(err);

        let (lite_block, err) = nu_parser::lite_parse(&lexed);
        error = error.or(err);

        let (mut block, err) =
            nu_parser::parse_block(&mut working_set, &lite_block, true, &[], false);
        error = error.or(err);

        if let Some(pipeline) = block.pipelines.get(1) {
            if let Some(expr) = pipeline.expressions.get(0) {
                return Err(ShellError::GenericError(
                    "error when loading nuon text".into(),
                    "could not load nuon text".into(),
                    Some(head),
                    None,
                    vec![ShellError::OutsideSpannedLabeledError(
                        string_input,
                        "error when loading".into(),
                        "excess values when loading".into(),
                        expr.span,
                    )],
                ));
            } else {
                return Err(ShellError::GenericError(
                    "error when loading nuon text".into(),
                    "could not load nuon text".into(),
                    Some(head),
                    None,
                    vec![ShellError::GenericError(
                        "error when loading".into(),
                        "excess values when loading".into(),
                        Some(head),
                        None,
                        Vec::new(),
                    )],
                ));
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

            if let Some(expr) = pipeline.expressions.get(1) {
                return Err(ShellError::GenericError(
                    "error when loading nuon text".into(),
                    "could not load nuon text".into(),
                    Some(head),
                    None,
                    vec![ShellError::OutsideSpannedLabeledError(
                        string_input,
                        "error when loading".into(),
                        "detected a pipeline in nuon file".into(),
                        expr.span,
                    )],
                ));
            }

            if pipeline.expressions.is_empty() {
                Expression {
                    expr: Expr::Nothing,
                    span: head,
                    custom_completion: None,
                    ty: Type::Nothing,
                }
            } else {
                pipeline.expressions.remove(0)
            }
        };

        if let Some(err) = error {
            return Err(ShellError::GenericError(
                "error when parsing nuon text".into(),
                "could not parse nuon text".into(),
                Some(head),
                None,
                vec![ShellError::OutsideSpannedLabeledError(
                    string_input,
                    "error when parsing".into(),
                    err.to_string(),
                    err.span(),
                )],
            ));
        }

        let result = convert_to_value(expr, head, &string_input);

        match result {
            Ok(result) => Ok(result.into_pipeline_data()),
            Err(err) => Err(ShellError::GenericError(
                "error when loading nuon text".into(),
                "could not load nuon text".into(),
                Some(head),
                None,
                vec![err],
            )),
        }
    }
}

fn convert_to_value(
    expr: Expression,
    span: Span,
    original_text: &str,
) -> Result<Value, ShellError> {
    match expr.expr {
        Expr::BinaryOp(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "binary operators not supported in nuon".into(),
            expr.span,
        )),
        Expr::UnaryNot(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "unary operators not supported in nuon".into(),
            expr.span,
        )),
        Expr::Block(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "blocks not supported in nuon".into(),
            expr.span,
        )),
        Expr::Closure(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "closures not supported in nuon".into(),
            expr.span,
        )),
        Expr::Binary(val) => Ok(Value::Binary { val, span }),
        Expr::Bool(val) => Ok(Value::Bool { val, span }),
        Expr::Call(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "calls not supported in nuon".into(),
            expr.span,
        )),
        Expr::CellPath(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "subexpressions and cellpaths not supported in nuon".into(),
            expr.span,
        )),
        Expr::DateTime(dt) => Ok(Value::Date { val: dt, span }),
        Expr::ExternalCall(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "calls not supported in nuon".into(),
            expr.span,
        )),
        Expr::Filepath(val) => Ok(Value::String { val, span }),
        Expr::Directory(val) => Ok(Value::String { val, span }),
        Expr::Float(val) => Ok(Value::Float { val, span }),
        Expr::FullCellPath(full_cell_path) => {
            if !full_cell_path.tail.is_empty() {
                Err(ShellError::OutsideSpannedLabeledError(
                    original_text.to_string(),
                    "Error when loading".into(),
                    "subexpressions and cellpaths not supported in nuon".into(),
                    expr.span,
                ))
            } else {
                convert_to_value(full_cell_path.head, span, original_text)
            }
        }

        Expr::Garbage => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "extra tokens in input file".into(),
            expr.span,
        )),
        Expr::GlobPattern(val) => Ok(Value::String { val, span }),
        Expr::ImportPattern(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "imports not supported in nuon".into(),
            expr.span,
        )),
        Expr::Overlay(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "overlays not supported in nuon".into(),
            expr.span,
        )),
        Expr::Int(val) => Ok(Value::Int { val, span }),
        Expr::Keyword(kw, ..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            format!("{} not supported in nuon", String::from_utf8_lossy(&kw)),
            expr.span,
        )),
        Expr::List(vals) => {
            let mut output = vec![];
            for val in vals {
                output.push(convert_to_value(val, span, original_text)?);
            }

            Ok(Value::List { vals: output, span })
        }
        Expr::Nothing => Ok(Value::Nothing { span }),
        Expr::Operator(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "operators not supported in nuon".into(),
            expr.span,
        )),
        Expr::Range(from, next, to, operator) => {
            let from = if let Some(f) = from {
                convert_to_value(*f, span, original_text)?
            } else {
                Value::Nothing { span: expr.span }
            };

            let next = if let Some(s) = next {
                convert_to_value(*s, span, original_text)?
            } else {
                Value::Nothing { span: expr.span }
            };

            let to = if let Some(t) = to {
                convert_to_value(*t, span, original_text)?
            } else {
                Value::Nothing { span: expr.span }
            };

            Ok(Value::Range {
                val: Box::new(Range::new(expr.span, from, next, to, &operator)?),
                span: expr.span,
            })
        }
        Expr::Record(key_vals) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for (key, val) in key_vals {
                let key_str = match key.expr {
                    Expr::String(key_str) => key_str,
                    _ => {
                        return Err(ShellError::OutsideSpannedLabeledError(
                            original_text.to_string(),
                            "Error when loading".into(),
                            "only strings can be keys".into(),
                            key.span,
                        ))
                    }
                };

                let value = convert_to_value(val, span, original_text)?;

                cols.push(key_str);
                vals.push(value);
            }

            Ok(Value::Record { cols, vals, span })
        }
        Expr::RowCondition(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "row conditions not supported in nuon".into(),
            expr.span,
        )),
        Expr::Signature(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "signatures not supported in nuon".into(),
            expr.span,
        )),
        Expr::String(s) => Ok(Value::String { val: s, span }),
        Expr::StringInterpolation(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "string interpolation not supported in nuon".into(),
            expr.span,
        )),
        Expr::Subexpression(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "subexpressions not supported in nuon".into(),
            expr.span,
        )),
        Expr::Table(headers, cells) => {
            let mut cols = vec![];

            let mut output = vec![];

            for key in headers {
                let key_str = match key.expr {
                    Expr::String(key_str) => key_str,
                    _ => {
                        return Err(ShellError::OutsideSpannedLabeledError(
                            original_text.to_string(),
                            "Error when loading".into(),
                            "only strings can be keys".into(),
                            expr.span,
                        ))
                    }
                };

                cols.push(key_str);
            }

            for row in cells {
                let mut vals = vec![];

                for cell in row {
                    vals.push(convert_to_value(cell, span, original_text)?);
                }

                if cols.len() != vals.len() {
                    return Err(ShellError::OutsideSpannedLabeledError(
                        original_text.to_string(),
                        "Error when loading".into(),
                        "table has mismatched columns".into(),
                        expr.span,
                    ));
                }

                output.push(Value::Record {
                    cols: cols.clone(),
                    vals,
                    span,
                });
            }

            Ok(Value::List { vals: output, span })
        }
        Expr::ValueWithUnit(val, unit) => {
            let size = match val.expr {
                Expr::Int(val) => val,
                _ => {
                    return Err(ShellError::OutsideSpannedLabeledError(
                        original_text.to_string(),
                        "Error when loading".into(),
                        "non-integer unit value".into(),
                        expr.span,
                    ))
                }
            };

            match unit.item {
                Unit::Byte => Ok(Value::Filesize { val: size, span }),
                Unit::Kilobyte => Ok(Value::Filesize {
                    val: size * 1000,
                    span,
                }),
                Unit::Megabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000,
                    span,
                }),
                Unit::Gigabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000 * 1000,
                    span,
                }),
                Unit::Terabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000 * 1000 * 1000,
                    span,
                }),
                Unit::Petabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                }),
                Unit::Exabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                }),
                Unit::Zettabyte => Ok(Value::Filesize {
                    val: size * 1000 * 1000 * 1000 * 1000 * 1000 * 1000 * 1000,
                    span,
                }),

                Unit::Kibibyte => Ok(Value::Filesize {
                    val: size * 1024,
                    span,
                }),
                Unit::Mebibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024,
                    span,
                }),
                Unit::Gibibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024 * 1024,
                    span,
                }),
                Unit::Tebibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024 * 1024 * 1024,
                    span,
                }),
                Unit::Pebibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                }),
                Unit::Exbibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                }),
                Unit::Zebibyte => Ok(Value::Filesize {
                    val: size * 1024 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024,
                    span,
                }),

                Unit::Nanosecond => Ok(Value::Duration { val: size, span }),
                Unit::Microsecond => Ok(Value::Duration {
                    val: size * 1000,
                    span,
                }),
                Unit::Millisecond => Ok(Value::Duration {
                    val: size * 1000 * 1000,
                    span,
                }),
                Unit::Second => Ok(Value::Duration {
                    val: size * 1000 * 1000 * 1000,
                    span,
                }),
                Unit::Minute => Ok(Value::Duration {
                    val: size * 1000 * 1000 * 1000 * 60,
                    span,
                }),
                Unit::Hour => Ok(Value::Duration {
                    val: size * 1000 * 1000 * 1000 * 60 * 60,
                    span,
                }),
                Unit::Day => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24) {
                    Some(val) => Ok(Value::Duration { val, span }),
                    None => Err(ShellError::OutsideSpannedLabeledError(
                        original_text.to_string(),
                        "day duration too large".into(),
                        "day duration too large".into(),
                        expr.span,
                    )),
                },

                Unit::Week => match size.checked_mul(1000 * 1000 * 1000 * 60 * 60 * 24 * 7) {
                    Some(val) => Ok(Value::Duration { val, span }),
                    None => Err(ShellError::OutsideSpannedLabeledError(
                        original_text.to_string(),
                        "week duration too large".into(),
                        "week duration too large".into(),
                        expr.span,
                    )),
                },
            }
        }
        Expr::Var(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "variables not supported in nuon".into(),
            expr.span,
        )),
        Expr::VarDecl(..) => Err(ShellError::OutsideSpannedLabeledError(
            original_text.to_string(),
            "Error when loading".into(),
            "variable declarations not supported in nuon".into(),
            expr.span,
        )),
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
