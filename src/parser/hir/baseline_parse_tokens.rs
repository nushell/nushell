use crate::context::Context;
use crate::errors::ShellError;
use crate::parser::{
    hir,
    hir::{
        baseline_parse_single_token, baseline_parse_token_as_number, baseline_parse_token_as_path,
        baseline_parse_token_as_pattern, baseline_parse_token_as_string,
    },
    DelimitedNode, Delimiter, PathNode, RawToken, TokenNode,
};
use crate::{Tag, Tagged, TaggedItem, Text};
use derive_new::new;
use log::trace;
use serde::{Deserialize, Serialize};

pub fn baseline_parse_tokens(
    token_nodes: &mut TokensIterator<'_>,
    context: &Context,
    source: &Text,
    syntax_type: SyntaxShape,
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum SyntaxShape {
    Any,
    List,
    Literal,
    String,
    Member,
    Variable,
    Number,
    Path,
    Pattern,
    Binary,
    Block,
    Boolean,
}

impl std::fmt::Display for SyntaxShape {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SyntaxShape::Any => write!(f, "Any"),
            SyntaxShape::List => write!(f, "List"),
            SyntaxShape::Literal => write!(f, "Literal"),
            SyntaxShape::String => write!(f, "String"),
            SyntaxShape::Member => write!(f, "Member"),
            SyntaxShape::Variable => write!(f, "Variable"),
            SyntaxShape::Number => write!(f, "Number"),
            SyntaxShape::Path => write!(f, "Path"),
            SyntaxShape::Pattern => write!(f, "Pattern"),
            SyntaxShape::Binary => write!(f, "Binary"),
            SyntaxShape::Block => write!(f, "Block"),
            SyntaxShape::Boolean => write!(f, "Boolean"),
        }
    }
}

pub fn baseline_parse_next_expr(
    tokens: &mut TokensIterator,
    context: &Context,
    source: &Text,
    syntax_type: SyntaxShape,
) -> Result<hir::Expression, ShellError> {
    let next = tokens
        .next()
        .ok_or_else(|| ShellError::string("Expected token, found none"))?;

    trace!(target: "nu::parser::parse_one_expr", "syntax_type={:?}, token={:?}", syntax_type, next);

    match (syntax_type, next) {
        (SyntaxShape::Path, TokenNode::Token(token)) => {
            return baseline_parse_token_as_path(token, context, source)
        }

        (SyntaxShape::Path, token) => {
            return Err(ShellError::type_error(
                "Path",
                token.type_name().tagged(token.tag()),
            ))
        }

        (SyntaxShape::Pattern, TokenNode::Token(token)) => {
            return baseline_parse_token_as_pattern(token, context, source)
        }

        (SyntaxShape::Pattern, token) => {
            return Err(ShellError::type_error(
                "Path",
                token.type_name().tagged(token.tag()),
            ))
        }

        (SyntaxShape::String, TokenNode::Token(token)) => {
            return baseline_parse_token_as_string(token, source);
        }

        (SyntaxShape::String, token) => {
            return Err(ShellError::type_error(
                "String",
                token.type_name().tagged(token.tag()),
            ))
        }

        (SyntaxShape::Number, TokenNode::Token(token)) => {
            return Ok(baseline_parse_token_as_number(token, source)?);
        }

        (SyntaxShape::Number, token) => {
            return Err(ShellError::type_error(
                "Numeric",
                token.type_name().tagged(token.tag()),
            ))
        }

        // TODO: More legit member processing
        (SyntaxShape::Member, TokenNode::Token(token)) => {
            return baseline_parse_token_as_string(token, source);
        }

        (SyntaxShape::Member, token) => {
            return Err(ShellError::type_error(
                "member",
                token.type_name().tagged(token.tag()),
            ))
        }

        (SyntaxShape::Any, _) => {}
        (SyntaxShape::List, _) => {}
        (SyntaxShape::Literal, _) => {}
        (SyntaxShape::Variable, _) => {}
        (SyntaxShape::Binary, _) => {}
        (SyntaxShape::Block, _) => {}
        (SyntaxShape::Boolean, _) => {}
    };

    let first = baseline_parse_semantic_token(next, context, source)?;

    let possible_op = tokens.peek();

    let op = match possible_op {
        Some(TokenNode::Operator(op)) => op.clone(),
        _ => return Ok(first),
    };

    tokens.next();

    let second = match tokens.next() {
        None => {
            return Err(ShellError::labeled_error(
                "Expected something after an operator",
                "operator",
                op.tag(),
            ))
        }
        Some(token) => baseline_parse_semantic_token(token, context, source)?,
    };

    // We definitely have a binary expression here -- let's see if we should coerce it into a block

    match syntax_type {
        SyntaxShape::Any => {
            let tag = first.tag().until(second.tag());
            let binary = hir::Binary::new(first, op, second);
            let binary = hir::RawExpression::Binary(Box::new(binary));
            let binary = binary.tagged(tag);

            Ok(binary)
        }

        SyntaxShape::Block => {
            let tag = first.tag().until(second.tag());

            let path: Tagged<hir::RawExpression> = match first {
                Tagged {
                    item: hir::RawExpression::Literal(hir::Literal::Bare),
                    tag,
                } => {
                    let string = tag.slice(source).to_string().tagged(tag);
                    let path = hir::Path::new(
                        // TODO: Deal with synthetic nodes that have no representation at all in source
                        hir::RawExpression::Variable(hir::Variable::It(Tag::unknown()))
                            .tagged(Tag::unknown()),
                        vec![string],
                    );
                    let path = hir::RawExpression::Path(Box::new(path));
                    path.tagged(first.tag())
                }
                Tagged {
                    item: hir::RawExpression::Literal(hir::Literal::String(inner)),
                    tag,
                } => {
                    let string = inner.slice(source).to_string().tagged(tag);
                    let path = hir::Path::new(
                        // TODO: Deal with synthetic nodes that have no representation at all in source
                        hir::RawExpression::Variable(hir::Variable::It(Tag::unknown()))
                            .tagged_unknown(),
                        vec![string],
                    );
                    let path = hir::RawExpression::Path(Box::new(path));
                    path.tagged(first.tag())
                }
                Tagged {
                    item: hir::RawExpression::Variable(..),
                    ..
                } => first,
                Tagged { tag, item } => {
                    return Err(ShellError::labeled_error(
                        "The first part of an un-braced block must be a column name",
                        item.type_name(),
                        tag,
                    ))
                }
            };

            let binary = hir::Binary::new(path, op, second);
            let binary = hir::RawExpression::Binary(Box::new(binary));
            let binary = binary.tagged(tag);

            let block = hir::RawExpression::Block(vec![binary]);
            let block = block.tagged(tag);

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
        TokenNode::Token(token) => baseline_parse_single_token(token, source),
        TokenNode::Call(_call) => unimplemented!(),
        TokenNode::Delimited(delimited) => baseline_parse_delimited(delimited, context, source),
        TokenNode::Pipeline(_pipeline) => unimplemented!(),
        TokenNode::Operator(op) => Err(ShellError::syntax_error(
            "Unexpected operator".tagged(op.tag),
        )),
        TokenNode::Flag(flag) => Err(ShellError::syntax_error("Unexpected flag".tagged(flag.tag))),
        TokenNode::Member(tag) => Err(ShellError::syntax_error(
            "BUG: Top-level member".tagged(*tag),
        )),
        TokenNode::Whitespace(tag) => Err(ShellError::syntax_error(
            "BUG: Whitespace found during parse".tagged(*tag),
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
                SyntaxShape::Any,
            )?;

            let expr = hir::RawExpression::Block(exprs);
            Ok(expr.tagged(token.tag()))
        }
        Delimiter::Paren => unimplemented!(),
        Delimiter::Square => {
            let children = token.children();
            let exprs = baseline_parse_tokens(
                &mut TokensIterator::new(children),
                context,
                source,
                SyntaxShape::Any,
            )?;

            let expr = hir::RawExpression::List(exprs);
            Ok(expr.tagged(token.tag()))
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
                RawToken::Bare => token.tag().slice(source),
                RawToken::String(tag) => tag.slice(source),
                RawToken::Number(_)
                | RawToken::Size(..)
                | RawToken::Variable(_)
                | RawToken::ExternalCommand(_)
                | RawToken::GlobPattern
                | RawToken::ExternalWord => {
                    return Err(ShellError::type_error(
                        "String",
                        token.type_name().tagged(part.tag()),
                    ))
                }
            },

            TokenNode::Member(tag) => tag.slice(source),

            // TODO: Make this impossible
            other => {
                return Err(ShellError::syntax_error(
                    format!("{} in path", other.type_name()).tagged(other.tag()),
                ))
            }
        }
        .to_string();

        tail.push(string.tagged(part.tag()));
    }

    Ok(hir::path(head, tail).tagged(token.tag()).into())
}

#[derive(Debug, new)]
pub struct TokensIterator<'a> {
    tokens: &'a [TokenNode],
    #[new(default)]
    index: usize,
    #[new(default)]
    seen: indexmap::IndexSet<usize>,
}

impl TokensIterator<'_> {
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

impl<'a> Iterator for TokensIterator<'a> {
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
