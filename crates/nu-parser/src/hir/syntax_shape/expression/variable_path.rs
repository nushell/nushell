use crate::hir::syntax_shape::{
    AnyExpressionShape, BareShape, ExpandSyntax, FlatShape, IntShape, ParseError, StringShape,
    WhitespaceShape,
};
use crate::hir::{Expression, SpannedExpression, TokensIterator};
use crate::parse::token_tree::{CompareOperatorType, DotDotType, DotType, ItVarType, VarType};
use crate::{hir, CompareOperator};
use nu_protocol::{PathMember, ShellTypeName};
use nu_source::{
    b, DebugDocBuilder, HasSpan, PrettyDebug, PrettyDebugWithSource, Span, Spanned, SpannedItem,
    Tag, Tagged, TaggedItem, Text,
};
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub struct VariablePathShape;

impl ExpandSyntax for VariablePathShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "variable path"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        // 1. let the head be the first token, expecting a variable
        // 2. let the tail be an empty list of members
        // 2. while the next token (excluding ws) is a dot:
        //   1. consume the dot
        //   2. consume the next token as a member and push it onto tail

        let head = token_nodes.expand_syntax(VariableShape)?;
        let start = head.span;
        let mut end = start;
        let mut tail: Vec<PathMember> = vec![];

        loop {
            match token_nodes.expand_syntax(DotShape) {
                Err(_) => break,
                Ok(_) => {}
            }

            let member = token_nodes.expand_syntax(MemberShape)?;
            let member = member.to_path_member(&token_nodes.source());

            end = member.span;
            tail.push(member);
        }

        Ok(Expression::path(head, tail).into_expr(start.until(end)))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PathTailShape;

#[derive(Debug, Clone)]
pub struct PathTailSyntax {
    pub tail: Vec<PathMember>,
    pub span: Span,
}

impl HasSpan for PathTailSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebug for PathTailSyntax {
    fn pretty(&self) -> DebugDocBuilder {
        b::typed("tail", b::intersperse(self.tail.iter(), b::space()))
    }
}

impl ExpandSyntax for PathTailShape {
    type Output = Result<PathTailSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "path continuation"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<PathTailSyntax, ParseError> {
        let mut end: Option<Span> = None;
        let mut tail: Vec<PathMember> = vec![];

        loop {
            match token_nodes.expand_syntax(DotShape) {
                Err(_) => break,
                Ok(_) => {}
            }

            let member = token_nodes.expand_syntax(MemberShape)?;
            let member = member.to_path_member(&token_nodes.source());
            end = Some(member.span);
            tail.push(member);
        }

        match end {
            None => Err(token_nodes.err_next_token("path continuation")),

            Some(end) => Ok(PathTailSyntax { tail, span: end }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContinuationSyntax {
    kind: ContinuationSyntaxKind,
    span: Span,
}

impl ContinuationSyntax {
    pub fn append_to(self, expr: SpannedExpression) -> SpannedExpression {
        match self.kind {
            ContinuationSyntaxKind::Infix(op, right) => {
                let span = expr.span.until(right.span);
                Expression::infix(expr, op, right).into_expr(span)
            }
            ContinuationSyntaxKind::Dot(_, member) => {
                let span = expr.span.until(member.span);
                Expression::dot_member(expr, member).into_expr(span)
            }
            ContinuationSyntaxKind::DotDot(_, right) => {
                let span = expr.span.until(right.span);
                Expression::range(expr, span, right).into_expr(span)
            }
        }
    }
}

impl HasSpan for ContinuationSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for ContinuationSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed("continuation", self.kind.pretty_debug(source))
    }
}

#[derive(Debug, Clone)]
pub enum ContinuationSyntaxKind {
    Infix(Spanned<CompareOperator>, SpannedExpression),
    Dot(Span, PathMember),
    DotDot(Span, SpannedExpression),
}

impl PrettyDebugWithSource for ContinuationSyntaxKind {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            ContinuationSyntaxKind::Infix(op, expr) => {
                b::operator(op.span.slice(source)) + expr.pretty_debug(source)
            }
            ContinuationSyntaxKind::Dot(span, member) => {
                b::operator(span.slice(source)) + member.pretty_debug(source)
            }
            ContinuationSyntaxKind::DotDot(span, expr) => {
                b::operator(span.slice(source)) + expr.pretty_debug(source)
            }
        }
    }
}

/// An expression continuation
#[derive(Debug, Copy, Clone)]
pub struct ExpressionContinuationShape;

impl ExpandSyntax for ExpressionContinuationShape {
    type Output = Result<ContinuationSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "expression continuation"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<ContinuationSyntax, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            // Try to expand a `.`
            let dot = token_nodes.expand_syntax(DotShape);

            match dot {
                // If a `.` was matched, it's a `Path`, and we expect a `Member` next
                Ok(dot) => {
                    let syntax = token_nodes.expand_syntax(MemberShape)?;
                    let member = syntax.to_path_member(&token_nodes.source());
                    let member_span = member.span;

                    return Ok(ContinuationSyntax {
                        kind: ContinuationSyntaxKind::Dot(dot, member),
                        span: dot.until(member_span),
                    });
                }

                Err(_) => {}
            }

            // Try to expand a `..`
            let dot = token_nodes.expand_syntax(DotDotShape);

            match dot {
                // If a `..` was matched, it's a `Range`, and we expect an `Expression` next
                Ok(dotdot) => {
                    let expr = token_nodes.expand_syntax(AnyExpressionShape)?;
                    let expr_span = expr.span;

                    return Ok(ContinuationSyntax {
                        kind: ContinuationSyntaxKind::DotDot(dotdot, expr),
                        span: dotdot.until(expr_span),
                    });
                }

                Err(_) => {}
            }

            // Otherwise, we expect an infix operator and an expression next
            let (_, op, _) = token_nodes.expand_syntax(InfixShape)?.infix.item;
            let next = token_nodes.expand_syntax(AnyExpressionShape)?;
            let next_span = next.span;

            return Ok(ContinuationSyntax {
                kind: ContinuationSyntaxKind::Infix(op.operator, next),
                span: op.operator.span.until(next_span),
            });
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VariableShape;

impl ExpandSyntax for VariableShape {
    type Output = Result<SpannedExpression, ParseError>;

    fn name(&self) -> &'static str {
        "variable"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
    ) -> Result<SpannedExpression, ParseError> {
        token_nodes
            .expand_token(ItVarType, |(inner, outer)| {
                Ok((
                    FlatShape::ItVariable,
                    Expression::it_variable(inner).into_expr(outer),
                ))
            })
            .or_else(|_| {
                token_nodes.expand_token(VarType, |(inner, outer)| {
                    Ok((
                        FlatShape::Variable,
                        Expression::variable(inner).into_expr(outer),
                    ))
                })
            })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Member {
    String(/* outer */ Span, /* inner */ Span),
    Int(BigInt, Span),
    Bare(Span),
}

impl ShellTypeName for Member {
    fn type_name(&self) -> &'static str {
        match self {
            Member::String(_, _) => "string",
            Member::Int(_, _) => "integer",
            Member::Bare(_) => "word",
        }
    }
}

impl Member {
    pub fn int(span: Span, source: &Text) -> Member {
        Member::Int(BigInt::from_str(span.slice(source)).unwrap(), span)
    }

    pub fn to_path_member(&self, source: &Text) -> PathMember {
        match self {
            Member::String(outer, inner) => PathMember::string(inner.slice(source), *outer),
            Member::Int(int, span) => PathMember::int(int.clone(), *span),
            Member::Bare(span) => PathMember::string(span.slice(source), *span),
        }
    }
}

impl PrettyDebugWithSource for Member {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            Member::String(outer, _) => b::value(outer.slice(source)),
            Member::Int(int, _) => b::value(format!("{}", int)),
            Member::Bare(span) => b::value(span.slice(source)),
        }
    }
}

impl HasSpan for Member {
    fn span(&self) -> Span {
        match self {
            Member::String(outer, ..) => *outer,
            Member::Int(_, int) => *int,
            Member::Bare(name) => *name,
        }
    }
}

impl Member {
    pub fn to_expr(&self) -> hir::SpannedExpression {
        match self {
            Member::String(outer, inner) => Expression::string(*inner).into_expr(outer),
            Member::Int(number, span) => Expression::number(number.clone()).into_expr(span),
            Member::Bare(span) => Expression::string(*span).into_expr(span),
        }
    }

    pub(crate) fn span(&self) -> Span {
        match self {
            Member::String(outer, _inner) => *outer,
            Member::Int(_, span) => *span,
            Member::Bare(span) => *span,
        }
    }
}

enum ColumnPathState {
    Initial,
    LeadingDot(Span),
    Dot(Span, Vec<Member>, Span),
    Member(Span, Vec<Member>),
    Error(ParseError),
}

impl ColumnPathState {
    pub fn dot(self, dot: Span) -> ColumnPathState {
        match self {
            ColumnPathState::Initial => ColumnPathState::LeadingDot(dot),
            ColumnPathState::LeadingDot(_) => {
                ColumnPathState::Error(ParseError::mismatch("column", "dot".spanned(dot)))
            }
            ColumnPathState::Dot(..) => {
                ColumnPathState::Error(ParseError::mismatch("column", "dot".spanned(dot)))
            }
            ColumnPathState::Member(tag, members) => ColumnPathState::Dot(tag, members, dot),
            ColumnPathState::Error(err) => ColumnPathState::Error(err),
        }
    }

    pub fn member(self, member: Member) -> ColumnPathState {
        match self {
            ColumnPathState::Initial => ColumnPathState::Member(member.span(), vec![member]),
            ColumnPathState::LeadingDot(tag) => {
                ColumnPathState::Member(tag.until(member.span()), vec![member])
            }

            ColumnPathState::Dot(tag, mut tags, _) => {
                ColumnPathState::Member(tag.until(member.span()), {
                    tags.push(member);
                    tags
                })
            }
            ColumnPathState::Member(..) => ColumnPathState::Error(ParseError::mismatch(
                "column",
                member.type_name().spanned(member.span()),
            )),
            ColumnPathState::Error(err) => ColumnPathState::Error(err),
        }
    }

    pub fn into_path(self, err: ParseError) -> Result<Tagged<Vec<Member>>, ParseError> {
        match self {
            ColumnPathState::Initial => Err(err),
            ColumnPathState::LeadingDot(dot) => {
                Err(ParseError::mismatch("column", "dot".spanned(dot)))
            }
            ColumnPathState::Dot(_tag, _members, dot) => {
                Err(ParseError::mismatch("column", "dot".spanned(dot)))
            }
            ColumnPathState::Member(tag, tags) => Ok(tags.tagged(tag)),
            ColumnPathState::Error(err) => Err(err),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ColumnPathShape;

impl ExpandSyntax for ColumnPathShape {
    type Output = Result<ColumnPathSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "column path"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<ColumnPathSyntax, ParseError> {
        let mut state = ColumnPathState::Initial;

        loop {
            let member = token_nodes.expand_syntax(MemberShape);

            match member {
                Err(_) => break,
                Ok(member) => state = state.member(member),
            }

            let dot = token_nodes.expand_syntax(DotShape);

            match dot {
                Err(_) => break,
                Ok(dot) => state = state.dot(dot),
            }
        }

        let path = state.into_path(token_nodes.err_next_token("column path"))?;

        Ok(ColumnPathSyntax {
            path: path.item,
            tag: path.tag,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ColumnPathSyntax {
    pub path: Vec<Member>,
    pub tag: Tag,
}

impl HasSpan for ColumnPathSyntax {
    fn span(&self) -> Span {
        self.tag.span
    }
}

impl PrettyDebugWithSource for ColumnPathSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "column path",
            b::intersperse(
                self.path.iter().map(|member| member.pretty_debug(source)),
                b::space(),
            ),
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MemberShape;

impl ExpandSyntax for MemberShape {
    type Output = Result<Member, ParseError>;

    fn name(&self) -> &'static str {
        "column"
    }

    fn expand<'a, 'b>(&self, token_nodes: &mut TokensIterator<'_>) -> Result<Member, ParseError> {
        if let Ok(int) = token_nodes.expand_syntax(IntMemberShape) {
            return Ok(int);
        }

        let bare = token_nodes.expand_syntax(BareShape);

        if let Ok(bare) = bare {
            return Ok(Member::Bare(bare.span()));
        }

        /* KATZ */
        /* let number = NumberShape.test(token_nodes, context);

        if let Some(peeked) = number {
            let node = peeked.not_eof("column")?.commit();
            let (n, span) = node.as_number().unwrap();

            return Ok(Member::Number(n, span))
        }*/

        let string = token_nodes.expand_syntax(StringShape);

        if let Ok(syntax) = string {
            return Ok(Member::String(syntax.span, syntax.inner));
        }

        Err(token_nodes.peek().type_error("column"))
    }
}

#[derive(Debug, Copy, Clone)]
struct IntMemberShape;

impl ExpandSyntax for IntMemberShape {
    type Output = Result<Member, ParseError>;

    fn name(&self) -> &'static str {
        "integer member"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<Member, ParseError> {
        token_nodes
            .expand_syntax(IntShape)
            .map(|int| Member::int(int.span(), &token_nodes.source()))
            .or_else(|_| Err(token_nodes.err_next_token("integer member")))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DotShape;

#[derive(Debug, Copy, Clone)]
pub struct ColorableDotShape;

impl ExpandSyntax for DotShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "dot"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(DotType, |token| Ok((FlatShape::Dot, token.span())))
    }
}

#[derive(Debug, Copy, Clone)]
struct DotDotShape;

impl ExpandSyntax for DotDotShape {
    type Output = Result<Span, ParseError>;

    fn name(&self) -> &'static str {
        "dotdot"
    }

    fn expand<'a, 'b>(&self, token_nodes: &'b mut TokensIterator<'a>) -> Result<Span, ParseError> {
        token_nodes.expand_token(DotDotType, |token| Ok((FlatShape::DotDot, token.span())))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InfixShape;

#[derive(Debug, Clone)]
pub struct InfixSyntax {
    infix: Spanned<(Span, InfixInnerSyntax, Span)>,
}

impl HasSpan for InfixSyntax {
    fn span(&self) -> Span {
        self.infix.span
    }
}

impl PrettyDebugWithSource for InfixSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.infix.1.pretty_debug(source)
    }
}

impl ExpandSyntax for InfixShape {
    type Output = Result<InfixSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "infix operator"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<InfixSyntax, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            // An infix operator must be prefixed by whitespace
            let start = token_nodes.expand_syntax(WhitespaceShape)?;

            // Parse the next TokenNode after the whitespace
            let operator = token_nodes.expand_syntax(InfixInnerShape)?;

            // An infix operator must be followed by whitespace
            let end = token_nodes.expand_syntax(WhitespaceShape)?;

            Ok(InfixSyntax {
                infix: (start, operator, end).spanned(start.until(end)),
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct InfixInnerSyntax {
    pub operator: Spanned<CompareOperator>,
}

impl HasSpan for InfixInnerSyntax {
    fn span(&self) -> Span {
        self.operator.span
    }
}

impl PrettyDebug for InfixInnerSyntax {
    fn pretty(&self) -> DebugDocBuilder {
        self.operator.pretty()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InfixInnerShape;

impl ExpandSyntax for InfixInnerShape {
    type Output = Result<InfixInnerSyntax, ParseError>;

    fn name(&self) -> &'static str {
        "infix inner"
    }

    fn expand<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
    ) -> Result<InfixInnerSyntax, ParseError> {
        token_nodes.expand_token(CompareOperatorType, |(span, operator)| {
            Ok((
                FlatShape::CompareOperator,
                InfixInnerSyntax {
                    operator: operator.spanned(span),
                },
            ))
        })
    }
}
