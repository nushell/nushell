use crate::errors::ShellError;
use crate::parser::{TokenNode, TokensIterator};
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
        TokenNode::Call(_call) => unimplemented!(),
        TokenNode::Nodes(_nodes) => unimplemented!(),
        TokenNode::Delimited(_delimited) => unimplemented!(),
        TokenNode::Pipeline(_pipeline) => unimplemented!(),
        TokenNode::Flag(flag) => flag.tag(),
        TokenNode::Member(member) => *member,
        TokenNode::Whitespace(_whitespace) => {
            unreachable!("This function should be called after next_non_ws()")
        }
        TokenNode::Error(_error) => unimplemented!(),
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
        TokenNode::Token(..) | TokenNode::Flag(..) | TokenNode::Member(..) => {}
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
