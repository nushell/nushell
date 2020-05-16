use nu_protocol::hir::*;
use nu_protocol::UnspannedPathMember;
use nu_source::{Spanned, SpannedItem};

/// Converts a SpannedExpression into a spanned shape(s) ready for color-highlighting
pub fn expression_to_flat_shape(e: &SpannedExpression) -> Vec<Spanned<FlatShape>> {
    match &e.expr {
        Expression::Block(exprs) => shapes(exprs),
        Expression::Invocation(exprs) => shapes(exprs),
        Expression::FilePath(_) => vec![FlatShape::Path.spanned(e.span)],
        Expression::Garbage => vec![FlatShape::Garbage.spanned(e.span)],
        Expression::List(exprs) => {
            let mut output = vec![];
            for expr in exprs.iter() {
                output.append(&mut expression_to_flat_shape(expr));
            }
            output
        }
        Expression::Path(exprs) => {
            let mut output = vec![];
            output.append(&mut expression_to_flat_shape(&exprs.head));
            for member in exprs.tail.iter() {
                if let UnspannedPathMember::String(_) = &member.unspanned {
                    output.push(FlatShape::StringMember.spanned(member.span));
                }
            }
            output
        }
        Expression::Command(command) => vec![FlatShape::InternalCommand.spanned(*command)],
        Expression::Literal(Literal::Bare(_)) => vec![FlatShape::BareMember.spanned(e.span)],
        Expression::Literal(Literal::ColumnPath(_)) => vec![FlatShape::Path.spanned(e.span)],
        Expression::Literal(Literal::GlobPattern(_)) => {
            vec![FlatShape::GlobPattern.spanned(e.span)]
        }
        Expression::Literal(Literal::Number(_)) => vec![FlatShape::Int.spanned(e.span)],
        Expression::Literal(Literal::Operator(_)) => vec![FlatShape::Operator.spanned(e.span)],
        Expression::Literal(Literal::Size(number, unit)) => vec![FlatShape::Size {
            number: number.span,
            unit: unit.span,
        }
        .spanned(e.span)],
        Expression::Literal(Literal::String(_)) => vec![FlatShape::String.spanned(e.span)],
        Expression::ExternalWord => vec![FlatShape::ExternalWord.spanned(e.span)],
        Expression::ExternalCommand(_) => vec![FlatShape::ExternalCommand.spanned(e.span)],
        Expression::Synthetic(_) => vec![FlatShape::BareMember.spanned(e.span)],
        Expression::Variable(_) => vec![FlatShape::Variable.spanned(e.span)],
        Expression::Binary(binary) => {
            let mut output = vec![];
            output.append(&mut expression_to_flat_shape(&binary.left));
            output.push(FlatShape::Operator.spanned(binary.op.span));
            output.append(&mut expression_to_flat_shape(&binary.right));
            output
        }
        Expression::Range(range) => {
            let mut output = vec![];
            output.append(&mut expression_to_flat_shape(&range.left));
            output.push(FlatShape::DotDot.spanned(range.dotdot));
            output.append(&mut expression_to_flat_shape(&range.right));
            output
        }
        Expression::Boolean(_) => vec![FlatShape::Keyword.spanned(e.span)],
    }
}

/// Converts a series of commands into a vec of spanned shapes ready for color-highlighting
pub fn shapes(commands: &Block) -> Vec<Spanned<FlatShape>> {
    let mut output = vec![];

    for pipeline in &commands.block {
        for command in &pipeline.list {
            match command {
                ClassifiedCommand::Internal(internal) => {
                    output.append(&mut expression_to_flat_shape(&internal.args.head));

                    if let Some(positionals) = &internal.args.positional {
                        for positional_arg in positionals {
                            output.append(&mut expression_to_flat_shape(positional_arg));
                        }
                    }

                    if let Some(named) = &internal.args.named {
                        for (_, named_arg) in named.iter() {
                            match named_arg {
                                NamedValue::PresentSwitch(span) => {
                                    output.push(FlatShape::Flag.spanned(*span));
                                }
                                NamedValue::Value(span, expr) => {
                                    output.push(FlatShape::Flag.spanned(*span));
                                    output.append(&mut expression_to_flat_shape(expr));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                ClassifiedCommand::Expr(expr) => output.append(&mut expression_to_flat_shape(expr)),
                _ => {}
            }
        }
    }

    output
}
