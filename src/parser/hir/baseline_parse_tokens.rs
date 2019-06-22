use crate::errors::ShellError;
use crate::parser::registry::CommandRegistry;
use crate::parser::{hir, hir::baseline_parse_single_token, Span, Spanned, TokenNode};
use crate::Text;

pub fn baseline_parse_tokens(
    token_nodes: &[TokenNode],
    registry: &dyn CommandRegistry,
    source: &Text,
) -> Result<Vec<hir::Expression>, ShellError> {
    let mut exprs: Vec<hir::Expression> = vec![];
    let mut rest = token_nodes;

    loop {
        if rest.len() == 0 {
            break;
        }

        let (expr, remainder) = baseline_parse_next_expr(rest, registry, source, None)?;
        exprs.push(expr);
        rest = remainder;
    }

    Ok(exprs)
}

#[allow(unused)]
#[derive(Debug)]
pub enum ExpressionKindHint {
    Literal,
    Variable,
    Binary,
    Block,
    Boolean,
}

pub fn baseline_parse_next_expr(
    token_nodes: &'nodes [TokenNode],
    _registry: &dyn CommandRegistry,
    source: &Text,
    coerce_hint: Option<ExpressionKindHint>,
) -> Result<(hir::Expression, &'nodes [TokenNode]), ShellError> {
    let mut tokens = token_nodes.iter().peekable();

    let first = next_token(&mut tokens);

    let first = match first {
        None => return Err(ShellError::unimplemented("Expected token, found none")),
        Some(token) => baseline_parse_semantic_token(token, source)?,
    };

    let possible_op = tokens.peek();

    let op = match possible_op {
        Some(TokenNode::Operator(op)) => op,
        _ => return Ok((first, &token_nodes[1..])),
    };

    tokens.next();

    let second = match tokens.next() {
        None => {
            return Err(ShellError::unimplemented(
                "Expected op followed by another expr, found nothing",
            ))
        }
        Some(token) => baseline_parse_semantic_token(token, source)?,
    };

    // We definitely have a binary expression here -- let's see if we should coerce it into a block

    match coerce_hint {
        None => {
            let span = (first.span.start, second.span.end);
            let binary = hir::Binary::new(first, *op, second);
            let binary = hir::RawExpression::Binary(Box::new(binary));
            let binary = Spanned::from_item(binary, span);

            Ok((binary, &token_nodes[3..]))
        }

        Some(hint) => match hint {
            ExpressionKindHint::Block => {
                let span = (first.span.start, second.span.end);

                let path: Spanned<hir::RawExpression> = match first {
                    Spanned {
                        item: hir::RawExpression::Literal(hir::Literal::Bare),
                        span,
                    } => {
                        let string = Spanned::from_item(span.slice(source).to_string(), span);
                        let path = hir::Path::new(
                            Spanned::from_item(
                                // TODO: Deal with synthetic nodes that have no representation at all in source
                                hir::RawExpression::Variable(hir::Variable::It(Span::from((0, 0)))),
                                (0, 0),
                            ),
                            vec![string],
                        );
                        let path = hir::RawExpression::Path(Box::new(path));
                        Spanned { item: path, span: first.span }
                    }
                    Spanned {
                        item: hir::RawExpression::Literal(hir::Literal::String(inner)),
                        span,
                    } => {
                        let string = Spanned::from_item(inner.slice(source).to_string(), span);
                        let path = hir::Path::new(
                            Spanned::from_item(
                                // TODO: Deal with synthetic nodes that have no representation at all in source
                                hir::RawExpression::Variable(hir::Variable::It(Span::from((0, 0)))),
                                (0, 0),
                            ),
                            vec![string],
                        );
                        let path = hir::RawExpression::Path(Box::new(path));
                        Spanned { item: path, span: first.span }
                    }
                    Spanned {
                        item: hir::RawExpression::Variable(..),
                        ..
                    } => first,
                    _ => {
                        return Err(ShellError::unimplemented(
                            "The first part of a block must be a string",
                        ))
                    }
                };

                let binary = hir::Binary::new(path, *op, second);
                let binary = hir::RawExpression::Binary(Box::new(binary));
                let binary = Spanned::from_item(binary, span);

                let block = hir::RawExpression::Block(Box::new(binary));
                let block = Spanned::from_item(block, span);

                Ok((block, &token_nodes[3..]))
            }

            other => unimplemented!("coerce hint {:?}", other),
        },
    }
}

pub fn baseline_parse_semantic_token(
    token: &TokenNode,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    match token {
        TokenNode::Token(token) => Ok(baseline_parse_single_token(token, source)),
        TokenNode::Call(_call) => unimplemented!(),
        TokenNode::Delimited(_delimited) => unimplemented!(),
        TokenNode::Pipeline(_pipeline) => unimplemented!(),
        TokenNode::Operator(_op) => unreachable!(),
        TokenNode::Flag(_flag) => unimplemented!(),
        TokenNode::Identifier(_span) => unreachable!(),
        TokenNode::Whitespace(_span) => unreachable!(),
        TokenNode::Error(error) => Err(*error.item.clone()),
        TokenNode::Path(_path) => unimplemented!(),
    }
}

fn next_token(nodes: &mut impl Iterator<Item = &'a TokenNode>) -> Option<&'a TokenNode> {
    loop {
        match nodes.next() {
            Some(TokenNode::Whitespace(_)) => continue,
            other => return other,
        }
    }
}
