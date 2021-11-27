use nu_protocol::ast::{
    Block, Expr, Expression, ImportPatternMember, PathMember, Pipeline, Statement,
};
use nu_protocol::{engine::StateWorkingSet, Span};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FlatShape {
    Garbage,
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
    Filepath,
    GlobPattern,
    Variable,
    Flag,
    Custom(String),
}

pub fn flatten_block(working_set: &StateWorkingSet, block: &Block) -> Vec<(Span, FlatShape)> {
    let mut output = vec![];
    for stmt in &block.stmts {
        output.extend(flatten_statement(working_set, stmt));
    }
    output
}

pub fn flatten_statement(
    working_set: &StateWorkingSet,
    stmt: &Statement,
) -> Vec<(Span, FlatShape)> {
    match stmt {
        Statement::Pipeline(pipeline) => flatten_pipeline(working_set, pipeline),
        _ => vec![],
    }
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
        Expr::Block(block_id) => flatten_block(working_set, working_set.get_block(*block_id)),
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
        Expr::ExternalCall(_, name_span, args) => {
            let mut output = vec![(*name_span, FlatShape::External)];

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
            let mut output = vec![];
            for l in list {
                output.extend(flatten_expression(working_set, l));
            }
            output
        }
        Expr::Record(list) => {
            let mut output = vec![];
            for l in list {
                output.extend(flatten_expression(working_set, &l.0));
                output.extend(flatten_expression(working_set, &l.1));
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
        Expr::RowCondition(block_id) | Expr::Subexpression(block_id) => {
            flatten_block(working_set, working_set.get_block(*block_id))
        }
        Expr::Table(headers, cells) => {
            let mut output = vec![];
            for e in headers {
                output.extend(flatten_expression(working_set, e));
            }
            for row in cells {
                for expr in row {
                    output.extend(flatten_expression(working_set, expr));
                }
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
