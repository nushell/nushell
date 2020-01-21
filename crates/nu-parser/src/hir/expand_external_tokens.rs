use crate::parse::token_tree::Token;
use crate::{
    hir::syntax_shape::{ExpandSyntax, FlatShape, MaybeSpaceShape},
    TokensIterator,
};
use derive_new::new;
use nu_errors::ParseError;
use nu_protocol::SpannedTypeName;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebug, Span, Spanned, SpannedItem};

#[derive(Debug, Eq, PartialEq, Clone, new)]
pub struct ExternalTokensSyntax {
    pub tokens: Spanned<Vec<Spanned<String>>>,
}

impl HasSpan for ExternalTokensSyntax {
    fn span(&self) -> Span {
        self.tokens.span
    }
}

impl PrettyDebug for ExternalTokensSyntax {
    fn pretty(&self) -> DebugDocBuilder {
        b::intersperse(
            self.tokens
                .iter()
                .map(|token| b::primitive(format!("{:?}", token.item))),
            b::space(),
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalTokensShape;

impl ExpandSyntax for ExternalTokensShape {
    type Output = ExternalTokensSyntax;

    fn name(&self) -> &'static str {
        "external tokens"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> ExternalTokensSyntax {
        let mut out: Vec<Spanned<String>> = vec![];

        let start = token_nodes.span_at_cursor();

        loop {
            match token_nodes.expand_syntax(ExternalExpressionShape) {
                Err(_) => break,
                Ok(span) => out.push(span.spanned_string(&token_nodes.source())),
            }
        }

        let end = token_nodes.span_at_cursor();

        ExternalTokensSyntax {
            tokens: out.spanned(start.until(end)),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExternalExpressionShape;

impl ExpandSyntax for ExternalExpressionShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "external expression"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_infallible(MaybeSpaceShape);

        let first = token_nodes.expand_syntax(ExternalStartToken)?;
        let mut last = first;

        loop {
            let continuation = token_nodes.expand_syntax(ExternalStartToken);

            if let Ok(continuation) = continuation {
                last = continuation;
            } else {
                break;
            }
        }

        Ok(first.until(last))
    }
}

#[derive(Debug, Copy, Clone)]
struct ExternalStartToken;

impl ExpandSyntax for ExternalStartToken {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "external start token"
    }
    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let mut span: Option<Span> = None;

            loop {
                let boundary = token_nodes.expand_infallible(PeekExternalBoundary);

                if boundary {
                    break;
                }

                let peeked = token_nodes.peek().not_eof("external start token")?;
                let node = peeked.node;

                let new_span = match node.unspanned() {
                    Token::Comment(_)
                    | Token::Separator
                    | Token::Whitespace
                    | Token::Pipeline(_) => {
                        return Err(ParseError::mismatch(
                            "external start token",
                            node.spanned_type_name(),
                        ))
                    }

                    _ => {
                        let node = peeked.commit();
                        node.span()
                    }
                };

                span = match span {
                    None => Some(new_span),
                    Some(before) => Some(before.until(new_span)),
                };
            }

            match span {
                None => Err(token_nodes.err_next_token("external start token")),
                Some(span) => {
                    token_nodes.color_shape(FlatShape::ExternalWord.spanned(span));
                    Ok(span)
                }
            }
        })
    }
}

#[derive(Debug, Copy, Clone)]
struct PeekExternalBoundary;

impl ExpandSyntax for PeekExternalBoundary {
    type Output = bool;

    fn name(&self) -> &'static str {
        "external boundary"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Self::Output {
        let next = token_nodes.peek();

        match next.node {
            None => true,
            Some(node) => match node.unspanned() {
                Token::Delimited(_) => true,
                Token::Whitespace => true,
                Token::Comment(_) => true,
                Token::Separator => true,
                Token::Call(_) => true,
                _ => false,
            },
        }
    }
}
