use crate::errors::ShellError;
use crate::parser::{
    hir::syntax_shape::{
        color_syntax, expand_atom, AtomicToken, ColorSyntax, ExpandContext, ExpansionRule,
        MaybeSpaceShape,
    },
    FlatShape, TokenNode, TokensIterator,
};
use crate::{Tag, Tagged, Text};

pub fn expand_external_tokens(
    token_nodes: &mut TokensIterator<'_>,
    source: &Text,
) -> Result<Vec<Tagged<String>>, ShellError> {
    let mut out: Vec<Tagged<String>> = vec![];

    loop {
        if let Some(tag) = expand_next_expression(token_nodes)? {
            out.push(tag.tagged_string(source));
        } else {
            break;
        }
    }

    Ok(out)
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalTokensShape;

impl ColorSyntax for ExternalTokensShape {
    type Info = ();
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> Self::Info {
        loop {
            // Allow a space
            color_syntax(&MaybeSpaceShape, token_nodes, context, shapes);

            // Process an external expression. External expressions are mostly words, with a
            // few exceptions (like $variables and path expansion rules)
            match color_syntax(&ExternalExpression, token_nodes, context, shapes).1 {
                ExternalExpressionResult::Eof => break,
                ExternalExpressionResult::Processed => continue,
            }
        }
    }
}

pub fn expand_next_expression(
    token_nodes: &mut TokensIterator<'_>,
) -> Result<Option<Tag>, ShellError> {
    let first = token_nodes.next_non_ws();

    let first = match first {
        None => return Ok(None),
        Some(v) => v,
    };

    let first = triage_external_head(first)?;
    let mut last = first;

    loop {
        let continuation = triage_continuation(token_nodes)?;

        if let Some(continuation) = continuation {
            last = continuation;
        } else {
            break;
        }
    }

    Ok(Some(first.until(last)))
}

fn triage_external_head(node: &TokenNode) -> Result<Tag, ShellError> {
    Ok(match node {
        TokenNode::Token(token) => token.tag(),
        TokenNode::Call(_call) => unimplemented!("TODO: OMG"),
        TokenNode::Nodes(_nodes) => unimplemented!("TODO: OMG"),
        TokenNode::Delimited(_delimited) => unimplemented!("TODO: OMG"),
        TokenNode::Pipeline(_pipeline) => unimplemented!("TODO: OMG"),
        TokenNode::Flag(flag) => flag.tag(),
        TokenNode::Whitespace(_whitespace) => {
            unreachable!("This function should be called after next_non_ws()")
        }
        TokenNode::Error(_error) => unimplemented!("TODO: OMG"),
    })
}

fn triage_continuation<'a, 'b>(
    nodes: &'a mut TokensIterator<'b>,
) -> Result<Option<Tag>, ShellError> {
    let mut peeked = nodes.peek_any();

    let node = match peeked.node {
        None => return Ok(None),
        Some(node) => node,
    };

    match &node {
        node if node.is_whitespace() => return Ok(None),
        TokenNode::Token(..) | TokenNode::Flag(..) => {}
        TokenNode::Call(..) => unimplemented!("call"),
        TokenNode::Nodes(..) => unimplemented!("nodes"),
        TokenNode::Delimited(..) => unimplemented!("delimited"),
        TokenNode::Pipeline(..) => unimplemented!("pipeline"),
        TokenNode::Whitespace(..) => unimplemented!("whitespace"),
        TokenNode::Error(..) => unimplemented!("error"),
    }

    peeked.commit();
    Ok(Some(node.tag()))
}

#[must_use]
enum ExternalExpressionResult {
    Eof,
    Processed,
}

#[derive(Debug, Copy, Clone)]
struct ExternalExpression;

impl ColorSyntax for ExternalExpression {
    type Info = ExternalExpressionResult;
    type Input = ();

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
        shapes: &mut Vec<Tagged<FlatShape>>,
    ) -> ExternalExpressionResult {
        let atom = match expand_atom(
            token_nodes,
            "external word",
            context,
            ExpansionRule::permissive(),
        ) {
            Err(_) => unreachable!("TODO: separate infallible expand_atom"),
            Ok(Tagged {
                item: AtomicToken::Eof { .. },
                ..
            }) => return ExternalExpressionResult::Eof,
            Ok(atom) => atom,
        };

        atom.color_tokens(shapes);
        return ExternalExpressionResult::Processed;
    }
}
