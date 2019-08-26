use crate::context::Context;
use crate::errors::ShellError;
use crate::parser::{
    hir,
    hir::{
        baseline_parse_single_token, baseline_parse_token_as_number, baseline_parse_token_as_path,
        baseline_parse_token_as_string,
    },
    parse::operator::Operator,
    DelimitedNode, Delimiter, PathNode, RawToken, TokenNode,
};
use crate::{Span, Tag, Tagged, TaggedItem, Text};
use derive_new::new;
use log::trace;
use serde::{Deserialize, Serialize};

pub fn baseline_parse_tokens(
    token_nodes: &mut TokensIterator<'_>,
    context: &Context,
    source: &Text,
    syntax_type: SyntaxType,
) -> Result<Vec<hir::Expression>, ShellError> {
    let mut exprs: Vec<hir::Expression> = vec![];

    loop {
        if token_nodes.at_end() {
            break;
        }

        let expr = baseline_parse_next_expr(token_nodes, context, source, syntax_type)?;
        exprs.push(expr);
    }

    Ok(exprs)
}

#[allow(unused)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SyntaxType {
    Any,
    List,
    Literal,
    String,
    Member,
    Variable,
    Number,
    Path,
    Binary,
    Block,
    Boolean,
}

pub fn baseline_parse_next_expr(
    tokens: &mut TokensIterator,
    context: &Context,
    source: &Text,
    syntax_type: SyntaxType,
) -> Result<hir::Expression, ShellError> {
    let next = tokens
        .next()
        .ok_or_else(|| ShellError::string("Expected token, found none"))?;

    trace!(target: "nu::parser::parse_one_expr", "syntax_type={:?}, token={:?}", syntax_type, next);

    match (syntax_type, next) {
        (SyntaxType::Path, TokenNode::Token(token)) => {
            return Ok(baseline_parse_token_as_path(token, context, source))
        }

        (SyntaxType::Path, token) => {
            return Err(ShellError::type_error(
                "Path",
                token.type_name().simple_spanned(token.span()),
            ))
        }

        (SyntaxType::String, TokenNode::Token(token)) => {
            return Ok(baseline_parse_token_as_string(token, source));
        }

        (SyntaxType::String, token) => {
            return Err(ShellError::type_error(
                "String",
                token.type_name().simple_spanned(token.span()),
            ))
        }

        (SyntaxType::Number, TokenNode::Token(token)) => {
            return Ok(baseline_parse_token_as_number(token, source));
        }

        (SyntaxType::Number, token) => {
            return Err(ShellError::type_error(
                "Numeric",
                token.type_name().simple_spanned(token.span()),
            ))
        }

        // TODO: More legit member processing
        (SyntaxType::Member, TokenNode::Token(token)) => {
            return Ok(baseline_parse_token_as_string(token, source));
        }

        (SyntaxType::Member, token) => {
            return Err(ShellError::type_error(
                "member",
                token.type_name().simple_spanned(token.span()),
            ))
        }

        (SyntaxType::Any, _) => {}
        (SyntaxType::List, _) => {}
        (SyntaxType::Literal, _) => {}
        (SyntaxType::Variable, _) => {}
        (SyntaxType::Binary, _) => {}
        (SyntaxType::Block, _) => {}
        (SyntaxType::Boolean, _) => {}
    };

    let possible_op = tokens.peek();
    match possible_op {
        Some(TokenNode::Operator(_)) => {},
        _ => return baseline_parse_semantic_token(next, context, source),
    }

    // We now know we have a boolean_expression that we need to begin parsing
    let mut expression_stack = Vec::new();
    let mut op_stack = Vec::new();

    expression_stack.push(baseline_parse_semantic_token(next, context, source)?);
    while match tokens.peek() {
        Some(TokenNode::Operator(_)) => true,
        _ => false,
    }{
        let op = match tokens.next() {
            Some(TokenNode::Operator(op)) => op.clone(),
            _ => panic!("Invariant violated: Next token was not available"),
        };

         match tokens.next() {
            None => {
                return Err(ShellError::labeled_error(
                    "Expected something after an operator",
                    "operator",
                    op.span(),
                ))
            }
            Some(token) => {
                op_stack.push(op);
                expression_stack.push(baseline_parse_semantic_token(token, context, source)?);
            },
        }
    };

    // We definitely have a binary expression here -- let's see if we should coerce it into a block

    let bin_expr = |first, second, op, span| {
        let binary = hir::Binary::new(first, op, second);
        let binary = hir::RawExpression::Binary(Box::new(binary));
        Tagged::from_simple_spanned_item(binary, span)
    };

    match syntax_type {
        SyntaxType::Any => {
            let second = expression_stack.pop().unwrap();
            let first = expression_stack.pop().unwrap();
            let op = op_stack.pop().unwrap();
            let span = (first.span().start, second.span().end);
            let end_of_expr = second.span().end;
            let mut binary = bin_expr(first, second, op, span);
            while let Some(ops) = op_stack.pop() {
                if ops.item == Operator::And {
                    let second = expression_stack.pop().unwrap();
                    let first = expression_stack.pop().unwrap();
                    let begin_of_expr = first.span().start;
                    let span = (first.span().start, second.span().end);
                    let op = op_stack.pop().unwrap();
                    let inner_bin = bin_expr(first, second, op, span);
                    binary = bin_expr(inner_bin, binary, ops, (begin_of_expr, end_of_expr));
                } else {
                    panic!("Invalid bool expr");
                }
            }


            Ok(binary)
        }
        SyntaxType::Block => {
            let second = expression_stack.pop().unwrap();
            let first = expression_stack.pop().unwrap();
            let op = op_stack.pop().unwrap();
            let span = (first.span().start, second.span().end);
            let mut begin_of_expr = first.span().end;
            let end_of_expr = second.span().end;
            let path = |first| match first {
                Tagged {
                    item: hir::RawExpression::Literal(hir::Literal::Bare),
                    tag: Tag { span, .. },
                } => {
                    let string =
                        Tagged::from_simple_spanned_item(span.slice(source).to_string(), span);
                    let path = hir::Path::new(
                        Tagged::from_simple_spanned_item(
                            // TODO: Deal with synthetic nodes that have no representation at all in source
                            hir::RawExpression::Variable(hir::Variable::It(Span::from((0, 0)))),
                            (0, 0),
                        ),
                        vec![string],
                    );
                    let path = hir::RawExpression::Path(Box::new(path));
                    Ok(Tagged::from_simple_spanned_item(path, first.span()))
                }
                Tagged {
                    item: hir::RawExpression::Literal(hir::Literal::String(inner)),
                    tag: Tag { span, .. },
                } => {
                    let string =
                        Tagged::from_simple_spanned_item(inner.slice(source).to_string(), span);
                    let path = hir::Path::new(
                        Tagged::from_simple_spanned_item(
                            // TODO: Deal with synthetic nodes that have no representation at all in source
                            hir::RawExpression::Variable(hir::Variable::It(Span::from((0, 0)))),
                            (0, 0),
                        ),
                        vec![string],
                    );
                    let path = hir::RawExpression::Path(Box::new(path));
                    Ok(Tagged::from_simple_spanned_item(path, first.span()))
                }
                Tagged {
                    item: hir::RawExpression::Variable(..),
                    ..
                } => Ok(first),
                Tagged {
                    tag: Tag { span, .. },
                    item,
                } => {
                    Err(ShellError::labeled_error(
                        "The first part of an un-braced block must be a column name",
                        item.type_name(),
                        span,
                    ))
                }
            };
            let mut binary = bin_expr(path(first)?, second, op, span);
            while let Some(ops) = op_stack.pop() {
                if ops.item == Operator::And {
                    let second = expression_stack.pop().unwrap();
                    let first = expression_stack.pop().unwrap();
                    begin_of_expr = first.span().start;
                    let span = (first.span().start, second.span().end);
                    let op = op_stack.pop().unwrap();
                    let inner_bin = bin_expr(path(first)?, second, op, span);
                    binary = bin_expr(inner_bin, binary, ops, (begin_of_expr, end_of_expr));
                } else {
                    panic!("Invalid bool expr");
                }
            }

            let block = hir::RawExpression::Block(vec![binary]);
            let block = Tagged::from_simple_spanned_item(block, (begin_of_expr, end_of_expr));
            Ok(block)
        }

        other => Err(ShellError::unimplemented(format!(
            "coerce hint {:?}",
            other
        ))),
    }
}

pub fn baseline_parse_semantic_token(
    token: &TokenNode,
    context: &Context,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    match token {
        TokenNode::Token(token) => Ok(baseline_parse_single_token(token, source)),
        TokenNode::Call(_call) => unimplemented!(),
        TokenNode::Delimited(delimited) => baseline_parse_delimited(delimited, context, source),
        TokenNode::Pipeline(_pipeline) => unimplemented!(),
        TokenNode::Operator(op) => Err(ShellError::syntax_error(
            "Unexpected operator".tagged(op.tag),
        )),
        TokenNode::Flag(flag) => Err(ShellError::syntax_error("Unexpected flag".tagged(flag.tag))),
        TokenNode::Member(span) => Err(ShellError::syntax_error(
            "BUG: Top-level member".tagged(span),
        )),
        TokenNode::Whitespace(span) => Err(ShellError::syntax_error(
            "BUG: Whitespace found during parse".tagged(span),
        )),
        TokenNode::Error(error) => Err(*error.item.clone()),
        TokenNode::Path(path) => baseline_parse_path(path, context, source),
    }
}

pub fn baseline_parse_delimited(
    token: &Tagged<DelimitedNode>,
    context: &Context,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    match token.delimiter() {
        Delimiter::Brace => {
            let children = token.children();
            let exprs = baseline_parse_tokens(
                &mut TokensIterator::new(children),
                context,
                source,
                SyntaxType::Any,
            )?;

            let expr = hir::RawExpression::Block(exprs);
            Ok(Tagged::from_simple_spanned_item(expr, token.span()))
        }
        Delimiter::Paren => unimplemented!(),
        Delimiter::Square => {
            let children = token.children();
            let exprs = baseline_parse_tokens(
                &mut TokensIterator::new(children),
                context,
                source,
                SyntaxType::Any,
            )?;

            let expr = hir::RawExpression::List(exprs);
            Ok(expr.tagged(Tag::unknown_origin(token.span())))
        }
    }
}

pub fn baseline_parse_path(
    token: &Tagged<PathNode>,
    context: &Context,
    source: &Text,
) -> Result<hir::Expression, ShellError> {
    let head = baseline_parse_semantic_token(token.head(), context, source)?;

    let mut tail = vec![];

    for part in token.tail() {
        let string = match part {
            TokenNode::Token(token) => match token.item() {
                RawToken::Bare => token.span().slice(source),
                RawToken::String(span) => span.slice(source),
                RawToken::Integer(_)
                | RawToken::Size(..)
                | RawToken::Variable(_)
                | RawToken::External(_) => {
                    return Err(ShellError::type_error(
                        "String",
                        token.type_name().simple_spanned(part),
                    ))
                }
            },

            TokenNode::Member(span) => span.slice(source),

            // TODO: Make this impossible
            other => {
                return Err(ShellError::syntax_error(
                    format!("{} in path", other.type_name()).tagged(other.span()),
                ))
            }
        }
        .to_string();

        tail.push(string.simple_spanned(part));
    }

    Ok(hir::path(head, tail).simple_spanned(token).into())
}

#[derive(Debug, new)]
pub struct TokensIterator<'a> {
    tokens: &'a [TokenNode],
    #[new(default)]
    index: usize,
    #[new(default)]
    seen: indexmap::IndexSet<usize>,
}

impl TokensIterator<'a> {
    pub fn remove(&mut self, position: usize) {
        self.seen.insert(position);
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn at_end(&self) -> bool {
        for index in self.index..self.tokens.len() {
            if !self.seen.contains(&index) {
                return false;
            }
        }

        true
    }

    pub fn advance(&mut self) {
        self.seen.insert(self.index);
        self.index += 1;
    }

    pub fn extract<T>(&mut self, f: impl Fn(&TokenNode) -> Option<T>) -> Option<(usize, T)> {
        for (i, item) in self.tokens.iter().enumerate() {
            if self.seen.contains(&i) {
                continue;
            }

            match f(item) {
                None => {
                    continue;
                }
                Some(value) => {
                    self.seen.insert(i);
                    return Some((i, value));
                }
            }
        }

        None
    }

    pub fn move_to(&mut self, pos: usize) {
        self.index = pos;
    }

    pub fn restart(&mut self) {
        self.index = 0;
    }

    pub fn clone(&self) -> TokensIterator {
        TokensIterator {
            tokens: self.tokens,
            index: self.index,
            seen: self.seen.clone(),
        }
    }

    pub fn peek(&self) -> Option<&TokenNode> {
        let mut tokens = self.clone();

        tokens.next()
    }

    pub fn debug_remaining(&self) -> Vec<TokenNode> {
        let mut tokens = self.clone();
        tokens.restart();
        tokens.cloned().collect()
    }
}

impl Iterator for TokensIterator<'a> {
    type Item = &'a TokenNode;

    fn next(&mut self) -> Option<&'a TokenNode> {
        loop {
            if self.index >= self.tokens.len() {
                return None;
            }

            if self.seen.contains(&self.index) {
                self.advance();
                continue;
            }

            if self.index >= self.tokens.len() {
                return None;
            }

            match &self.tokens[self.index] {
                TokenNode::Whitespace(_) => {
                    self.advance();
                }
                other => {
                    self.advance();
                    return Some(other);
                }
            }
        }
    }
}
