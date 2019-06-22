use crate::errors::ShellError;
use crate::parser::registry::CommandRegistry;
use crate::parser::{hir, hir::baseline_parse_single_token, Span, Spanned, TokenNode};

pub fn baseline_parse_tokens(
    token_nodes: &[TokenNode],
    registry: &dyn CommandRegistry,
    source: &str,
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
    source: &str,
    coerce_hint: Option<ExpressionKindHint>,
) -> Result<(hir::Expression, &'nodes [TokenNode]), ShellError> {
    println!(
        "baseline_parse_next_expr {:?} - {:?}",
        token_nodes, coerce_hint
    );

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

                let string: Spanned<String> = match first {
                    Spanned {
                        item: hir::RawExpression::Literal(hir::Literal::Bare),
                        span,
                    } => Spanned::from_item(span.slice(source).to_string(), span),
                    Spanned {
                        item: hir::RawExpression::Literal(hir::Literal::String(inner)),
                        span,
                    } => Spanned::from_item(inner.slice(source).to_string(), span),
                    _ => {
                        return Err(ShellError::unimplemented(
                            "The first part of a block must be a string",
                        ))
                    }
                };

                let path = hir::Path::new(
                    Spanned::from_item(
                        // TODO: Deal with synthetic nodes that have no representation at all in source
                        hir::RawExpression::Variable(hir::Variable::It(Span::from((0, 0)))),
                        (0, 0),
                    ),
                    vec![string],
                );
                let path = hir::RawExpression::Path(Box::new(path));
                let path = Spanned::from_item(path, first.span);

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
    source: &str,
) -> Result<hir::Expression, ShellError> {
    match token {
        TokenNode::Token(token) => Ok(baseline_parse_single_token(token, source)),
        TokenNode::Call(call) => unimplemented!(),
        TokenNode::Delimited(delimited) => unimplemented!(),
        TokenNode::Pipeline(pipeline) => unimplemented!(),
        TokenNode::Operator(_op) => unreachable!(),
        TokenNode::Flag(flag) => unimplemented!(),
        TokenNode::Identifier(_span) => unreachable!(),
        TokenNode::Whitespace(_span) => unreachable!(),
        TokenNode::Error(error) => Err(*error.item.clone()),
        TokenNode::Path(path) => unimplemented!(),
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

fn baseline_parse_token(
    token_node: &TokenNode,
    _registry: &dyn CommandRegistry,
    source: &str,
) -> Result<hir::Expression, ShellError> {
    match token_node {
        TokenNode::Token(token) => Ok(hir::baseline_parse_single_token(token, source)),
        TokenNode::Call(_call) => Err(ShellError::unimplemented("baseline_parse Call")),
        TokenNode::Delimited(_delimited) => {
            Err(ShellError::unimplemented("baseline_parse Delimited"))
        }
        TokenNode::Pipeline(_pipeline) => Err(ShellError::unimplemented("baseline_parse Pipeline")),
        TokenNode::Path(_path) => Err(ShellError::unimplemented("baseline_parse Path")),
        TokenNode::Operator(_op) => Err(ShellError::unimplemented("baseline_parse Operator")),
        TokenNode::Flag(_op) => Err(ShellError::unimplemented("baseline_parse Flag")),
        TokenNode::Identifier(_op) => Err(ShellError::unimplemented("baseline_parse Identifier")),
        TokenNode::Whitespace(_op) => Err(ShellError::unimplemented("baseline_parse Whitespace")),
        TokenNode::Error(err) => Err(*err.item.clone()),
    }
}
