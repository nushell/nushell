use crate::hir::syntax_shape::{ExpandSyntax, FlatShape, NumberShape, VariableShape};
use crate::hir::TokensIterator;
use crate::hir::{Expression, SpannedExpression};
use crate::parse::token_tree::{BareType, StrType};
use nu_errors::ParseError;
use nu_source::{b, DebugDocBuilder, HasSpan, PrettyDebugWithSource, Span};

#[derive(Debug, Copy, Clone)]
pub struct CoerceStringShape;

impl ExpandSyntax for CoerceStringShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "StringShape"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_token(StrType, |(s, outer)| {
                Ok((FlatShape::String, Expression::str(s).into_expr(outer)))
            })
            .or_else(|_| {
                token_nodes.expand_token(BareType, |span| {
                    Ok((FlatShape::String, Expression::string(span).into_expr(span)))
                })
            })
            .or_else(|_| {
                #[allow(deprecated)]
                token_nodes
                    .expand_syntax(NumberShape)
                    .map(|number| Expression::string(number.span()).into_expr(number.span()))
            })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct StringExpressionShape;

impl ExpandSyntax for StringExpressionShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "string"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes.expand_syntax(VariableShape).or_else(|_| {
            token_nodes.expand_token(StrType, |(s, outer)| {
                Ok((FlatShape::String, Expression::str(s).into_expr(outer)))
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct StringSyntax {
    pub inner: Span,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StrSyntax {
    pub content: String,
    pub span: Span,
}

impl HasSpan for StrSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for StrSyntax {
    fn pretty_debug(&self, _: &str) -> DebugDocBuilder {
        b::primitive(self.content.clone())
    }
}

impl HasSpan for StringSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for StringSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::primitive(self.span.slice(source))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct StringShape;

impl ExpandSyntax for StringShape {
    type Output = Result<StrSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "string"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<StrSyntax, ParseError> {
        token_nodes.expand_token(StrType, |(s, outer)| {
            Ok((
                FlatShape::String,
                StrSyntax {
                    content: s,
                    span: outer,
                },
            ))
        })
    }
}
