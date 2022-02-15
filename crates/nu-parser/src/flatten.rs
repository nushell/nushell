use nu_protocol::ast::{Block, Expr, Expression, ImportPatternMember, PathMember, Pipeline};
use nu_protocol::{engine::StateWorkingSet, Span};
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FlatShape {
    Garbage,
    Nothing,
    Bool,
    Int,
    Float,
    Range,
    InternalCall,
    External,
    ExternalArg,
    Literal,
    Operator,
    Signature,
    String,
    StringInterpolation,
    List,
    Table,
    Record,
    Block,
    Filepath,
    GlobPattern,
    Variable,
    Flag,
    Custom(String),
}

impl Display for FlatShape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            FlatShape::Garbage => write!(f, "flatshape_garbage"),
            FlatShape::Nothing => write!(f, "flatshape_nothing"),
            FlatShape::Bool => write!(f, "flatshape_bool"),
            FlatShape::Int => write!(f, "flatshape_int"),
            FlatShape::Float => write!(f, "flatshape_float"),
            FlatShape::Range => write!(f, "flatshape_range"),
            FlatShape::InternalCall => write!(f, "flatshape_internalcall"),
            FlatShape::External => write!(f, "flatshape_external"),
            FlatShape::ExternalArg => write!(f, "flatshape_externalarg"),
            FlatShape::Literal => write!(f, "flatshape_literal"),
            FlatShape::Operator => write!(f, "flatshape_operator"),
            FlatShape::Signature => write!(f, "flatshape_signature"),
            FlatShape::String => write!(f, "flatshape_string"),
            FlatShape::StringInterpolation => write!(f, "flatshape_string_interpolation"),
            FlatShape::List => write!(f, "flatshape_string_interpolation"),
            FlatShape::Table => write!(f, "flatshape_table"),
            FlatShape::Record => write!(f, "flatshape_record"),
            FlatShape::Block => write!(f, "flatshape_block"),
            FlatShape::Filepath => write!(f, "flatshape_filepath"),
            FlatShape::GlobPattern => write!(f, "flatshape_globpattern"),
            FlatShape::Variable => write!(f, "flatshape_variable"),
            FlatShape::Flag => write!(f, "flatshape_flag"),
            FlatShape::Custom(_) => write!(f, "flatshape_custom"),
        }
    }
}

pub fn flatten_block(working_set: &StateWorkingSet, block: &Block) -> Vec<(Span, FlatShape)> {
    let mut output = vec![];
    for pipeline in &block.pipelines {
        output.extend(flatten_pipeline(working_set, pipeline));
    }
    output
}

pub fn flatten_expression(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Vec<(Span, FlatShape)> {
    if let Some(custom_completion) = &expr.custom_completion {
        return vec![(expr.span, FlatShape::Custom(custom_completion.clone()))];
    }

    match &expr.expr {
        Expr::BinaryOp(lhs, op, rhs) => {
            let mut output = vec![];
            output.extend(flatten_expression(working_set, lhs));
            output.extend(flatten_expression(working_set, op));
            output.extend(flatten_expression(working_set, rhs));
            output
        }
        Expr::Block(block_id) | Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            let outer_span = expr.span;

            let mut output = vec![];

            let flattened = flatten_block(working_set, working_set.get_block(*block_id));

            if let Some(first) = flattened.first() {
                if first.0.start > outer_span.start {
                    output.push((
                        Span {
                            start: outer_span.start,
                            end: first.0.start,
                        },
                        FlatShape::Block,
                    ));
                }
            }

            let last = if let Some(last) = flattened.last() {
                if last.0.end < outer_span.end {
                    Some((
                        Span {
                            start: last.0.end,
                            end: outer_span.end,
                        },
                        FlatShape::Block,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            output.extend(flattened);
            if let Some(last) = last {
                output.push(last)
            }

            output
        }
        Expr::Call(call) => {
            let mut output = vec![(call.head, FlatShape::InternalCall)];

            let mut args = vec![];
            for positional in &call.positional {
                args.extend(flatten_expression(working_set, positional));
            }
            for named in &call.named {
                args.push((named.0.span, FlatShape::Flag));
                if let Some(expr) = &named.1 {
                    args.extend(flatten_expression(working_set, expr));
                }
            }
            // sort these since flags and positional args can be intermixed
            args.sort();

            output.extend(args);
            output
        }
        Expr::ExternalCall(head, args) => {
            let mut output = vec![];

            match **head {
                Expression {
                    expr: Expr::String(..),
                    span,
                    ..
                } => {
                    output.push((span, FlatShape::External));
                }
                _ => {
                    output.extend(flatten_expression(working_set, head));
                }
            }

            for arg in args {
                //output.push((*arg, FlatShape::ExternalArg));
                match arg {
                    Expression {
                        expr: Expr::String(..),
                        span,
                        ..
                    } => {
                        output.push((*span, FlatShape::ExternalArg));
                    }
                    _ => {
                        output.extend(flatten_expression(working_set, arg));
                    }
                }
            }

            output
        }
        Expr::Garbage => {
            vec![(expr.span, FlatShape::Garbage)]
        }
        Expr::Nothing => {
            vec![(expr.span, FlatShape::Nothing)]
        }
        Expr::Int(_) => {
            vec![(expr.span, FlatShape::Int)]
        }
        Expr::Float(_) => {
            vec![(expr.span, FlatShape::Float)]
        }
        Expr::ValueWithUnit(x, unit) => {
            let mut output = flatten_expression(working_set, x);
            output.push((unit.span, FlatShape::String));

            output
        }
        Expr::CellPath(cell_path) => {
            let mut output = vec![];
            for path_element in &cell_path.members {
                match path_element {
                    PathMember::String { span, .. } => output.push((*span, FlatShape::String)),
                    PathMember::Int { span, .. } => output.push((*span, FlatShape::Int)),
                }
            }
            output
        }
        Expr::FullCellPath(cell_path) => {
            let mut output = vec![];
            output.extend(flatten_expression(working_set, &cell_path.head));
            for path_element in &cell_path.tail {
                match path_element {
                    PathMember::String { span, .. } => output.push((*span, FlatShape::String)),
                    PathMember::Int { span, .. } => output.push((*span, FlatShape::Int)),
                }
            }
            output
        }
        Expr::ImportPattern(import_pattern) => {
            let mut output = vec![(import_pattern.head.span, FlatShape::String)];

            for member in &import_pattern.members {
                match member {
                    ImportPatternMember::Glob { span } => output.push((*span, FlatShape::String)),
                    ImportPatternMember::Name { span, .. } => {
                        output.push((*span, FlatShape::String))
                    }
                    ImportPatternMember::List { names } => {
                        for (_, span) in names {
                            output.push((*span, FlatShape::String));
                        }
                    }
                }
            }

            output
        }
        Expr::Range(from, next, to, op) => {
            let mut output = vec![];
            if let Some(f) = from {
                output.extend(flatten_expression(working_set, f));
            }
            if let Some(s) = next {
                output.extend(vec![(op.next_op_span, FlatShape::Operator)]);
                output.extend(flatten_expression(working_set, s));
            }
            output.extend(vec![(op.span, FlatShape::Operator)]);
            if let Some(t) = to {
                output.extend(flatten_expression(working_set, t));
            }
            output
        }
        Expr::Bool(_) => {
            vec![(expr.span, FlatShape::Bool)]
        }
        Expr::Filepath(_) => {
            vec![(expr.span, FlatShape::Filepath)]
        }
        Expr::GlobPattern(_) => {
            vec![(expr.span, FlatShape::GlobPattern)]
        }
        Expr::List(list) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            let mut output = vec![];
            for l in list {
                let flattened = flatten_expression(working_set, l);

                if let Some(first) = flattened.first() {
                    if first.0.start > last_end {
                        output.push((
                            Span {
                                start: last_end,
                                end: first.0.start,
                            },
                            FlatShape::List,
                        ));
                    }
                }

                if let Some(last) = flattened.last() {
                    last_end = last.0.end;
                }

                output.extend(flattened);
            }

            if last_end < outer_span.end {
                output.push((
                    Span {
                        start: last_end,
                        end: outer_span.end,
                    },
                    FlatShape::List,
                ));
            }
            output
        }
        Expr::StringInterpolation(exprs) => {
            let mut output = vec![(
                Span {
                    start: expr.span.start,
                    end: expr.span.start + 2,
                },
                FlatShape::StringInterpolation,
            )];
            for expr in exprs {
                output.extend(flatten_expression(working_set, expr));
            }
            output.push((
                Span {
                    start: expr.span.end - 1,
                    end: expr.span.end,
                },
                FlatShape::StringInterpolation,
            ));
            output
        }
        Expr::Record(list) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            let mut output = vec![];
            for l in list {
                let flattened_lhs = flatten_expression(working_set, &l.0);
                let flattened_rhs = flatten_expression(working_set, &l.1);

                if let Some(first) = flattened_lhs.first() {
                    if first.0.start > last_end {
                        output.push((
                            Span {
                                start: last_end,
                                end: first.0.start,
                            },
                            FlatShape::Record,
                        ));
                    }
                }
                if let Some(last) = flattened_lhs.last() {
                    last_end = last.0.end;
                }
                output.extend(flattened_lhs);

                if let Some(first) = flattened_rhs.first() {
                    if first.0.start > last_end {
                        output.push((
                            Span {
                                start: last_end,
                                end: first.0.start,
                            },
                            FlatShape::Record,
                        ));
                    }
                }
                if let Some(last) = flattened_rhs.last() {
                    last_end = last.0.end;
                }

                output.extend(flattened_rhs);
            }
            if last_end < outer_span.end {
                output.push((
                    Span {
                        start: last_end,
                        end: outer_span.end,
                    },
                    FlatShape::Record,
                ));
            }

            output
        }
        Expr::Keyword(_, span, expr) => {
            let mut output = vec![(*span, FlatShape::InternalCall)];
            output.extend(flatten_expression(working_set, expr));
            output
        }
        Expr::Operator(_) => {
            vec![(expr.span, FlatShape::Operator)]
        }
        Expr::Signature(_) => {
            vec![(expr.span, FlatShape::Signature)]
        }
        Expr::String(_) => {
            vec![(expr.span, FlatShape::String)]
        }
        Expr::Table(headers, cells) => {
            let outer_span = expr.span;
            let mut last_end = outer_span.start;

            let mut output = vec![];
            for e in headers {
                let flattened = flatten_expression(working_set, e);
                if let Some(first) = flattened.first() {
                    if first.0.start > last_end {
                        output.push((
                            Span {
                                start: last_end,
                                end: first.0.start,
                            },
                            FlatShape::Table,
                        ));
                    }
                }

                if let Some(last) = flattened.last() {
                    last_end = last.0.end;
                }

                output.extend(flattened);
            }
            for row in cells {
                for expr in row {
                    let flattened = flatten_expression(working_set, expr);
                    if let Some(first) = flattened.first() {
                        if first.0.start > last_end {
                            output.push((
                                Span {
                                    start: last_end,
                                    end: first.0.start,
                                },
                                FlatShape::Table,
                            ));
                        }
                    }

                    if let Some(last) = flattened.last() {
                        last_end = last.0.end;
                    }

                    output.extend(flattened);
                }
            }

            if last_end < outer_span.end {
                output.push((
                    Span {
                        start: last_end,
                        end: outer_span.end,
                    },
                    FlatShape::Table,
                ));
            }

            output
        }
        Expr::Var(_) | Expr::VarDecl(_) => {
            vec![(expr.span, FlatShape::Variable)]
        }
    }
}

pub fn flatten_pipeline(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
) -> Vec<(Span, FlatShape)> {
    let mut output = vec![];
    for expr in &pipeline.expressions {
        output.extend(flatten_expression(working_set, expr))
    }
    output
}
