use crate::hir::syntax_shape::{
    color_fallible_syntax, color_fallible_syntax_with, expand_atom, expand_expr, expand_syntax,
    parse_single_node, AnyExpressionShape, BareShape, ExpandContext, ExpandExpression,
    ExpandSyntax, ExpansionRule, FallibleColorSyntax, FlatShape, ParseError, Peeked, SkipSyntax,
    StringShape, TestSyntax, UnspannedAtomicToken, WhitespaceShape,
};
use crate::parse::tokens::{RawNumber, UnspannedToken};
use crate::{hir, hir::Expression, hir::TokensIterator, CompareOperator, EvaluationOperator};
use nu_errors::ShellError;
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

impl ExpandExpression for VariablePathShape {
    fn name(&self) -> &'static str {
        "variable path"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        // 1. let the head be the first token, expecting a variable
        // 2. let the tail be an empty list of members
        // 2. while the next token (excluding ws) is a dot:
        //   1. consume the dot
        //   2. consume the next token as a member and push it onto tail

        let head = expand_expr(&VariableShape, token_nodes, context)?;
        let start = head.span;
        let mut end = start;
        let mut tail: Vec<PathMember> = vec![];

        loop {
            if DotShape.skip(token_nodes, context).is_err() {
                break;
            }

            let member = expand_syntax(&MemberShape, token_nodes, context)?;
            let member = member.to_path_member(context.source);

            end = member.span;
            tail.push(member);
        }

        Ok(hir::Expression::path(head, tail, start.until(end)))
    }
}

impl FallibleColorSyntax for VariablePathShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "VariablePathShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        token_nodes.atomic(|token_nodes| {
            // If the head of the token stream is not a variable, fail
            color_fallible_syntax(&VariableShape, token_nodes, context)?;

            loop {
                // look for a dot at the head of a stream
                if color_fallible_syntax_with(
                    &ColorableDotShape,
                    &FlatShape::Dot,
                    token_nodes,
                    context,
                )
                .is_err()
                {
                    // if there's no dot, we're done
                    break;
                }

                // otherwise, look for a member, and if you don't find one, fail
                color_fallible_syntax(&MemberShape, token_nodes, context)?;
            }

            Ok(())
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PathTailShape;

/// The failure mode of `PathTailShape` is a dot followed by a non-member
impl FallibleColorSyntax for PathTailShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "PathTailShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        token_nodes.atomic(|token_nodes| loop {
            let result = color_fallible_syntax_with(
                &ColorableDotShape,
                &FlatShape::Dot,
                token_nodes,
                context,
            );

            if result.is_err() {
                return Ok(());
            }

            // If we've seen a dot but not a member, fail
            color_fallible_syntax(&MemberShape, token_nodes, context)?;
        })
    }
}

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
    type Output = PathTailSyntax;

    fn name(&self) -> &'static str {
        "path continuation"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let mut end: Option<Span> = None;
        let mut tail: Vec<PathMember> = vec![];

        loop {
            if DotShape.skip(token_nodes, context).is_err() {
                break;
            }

            let member = expand_syntax(&MemberShape, token_nodes, context)?;
            let member = member.to_path_member(context.source);
            end = Some(member.span);
            tail.push(member);
        }

        match end {
            None => Err(ParseError::mismatch(
                "path tail",
                token_nodes.typed_span_at_cursor(),
            )),

            Some(end) => Ok(PathTailSyntax { tail, span: end }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpressionContinuation {
    DotSuffix(Span, PathMember),
    InfixSuffix(Spanned<CompareOperator>, Expression),
}

impl PrettyDebugWithSource for ExpressionContinuation {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            ExpressionContinuation::DotSuffix(_, suffix) => {
                b::operator(".") + suffix.pretty_debug(source)
            }
            ExpressionContinuation::InfixSuffix(op, expr) => {
                op.pretty_debug(source) + b::space() + expr.pretty_debug(source)
            }
        }
    }
}

impl HasSpan for ExpressionContinuation {
    fn span(&self) -> Span {
        match self {
            ExpressionContinuation::DotSuffix(dot, column) => dot.until(column.span),
            ExpressionContinuation::InfixSuffix(operator, expression) => {
                operator.span.until(expression.span)
            }
        }
    }
}

/// An expression continuation
#[derive(Debug, Copy, Clone)]
pub struct ExpressionContinuationShape;

impl ExpandSyntax for ExpressionContinuationShape {
    type Output = ExpressionContinuation;

    fn name(&self) -> &'static str {
        "expression continuation"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<ExpressionContinuation, ParseError> {
        // Try to expand a `.`
        let dot = expand_syntax(&DotShape, token_nodes, context);

        match dot {
            // If a `.` was matched, it's a `Path`, and we expect a `Member` next
            Ok(dot) => {
                let syntax = expand_syntax(&MemberShape, token_nodes, context)?;
                let member = syntax.to_path_member(context.source);

                Ok(ExpressionContinuation::DotSuffix(dot, member))
            }

            // Otherwise, we expect an infix operator and an expression next
            Err(_) => {
                let (_, op, _) = expand_syntax(&InfixShape, token_nodes, context)?.infix.item;
                let next = expand_expr(&AnyExpressionShape, token_nodes, context)?;

                Ok(ExpressionContinuation::InfixSuffix(op.operator, next))
            }
        }
    }
}

pub enum ContinuationInfo {
    Dot,
    Infix,
}

impl FallibleColorSyntax for ExpressionContinuationShape {
    type Info = ContinuationInfo;
    type Input = ();

    fn name(&self) -> &'static str {
        "ExpressionContinuationShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<ContinuationInfo, ShellError> {
        token_nodes.atomic(|token_nodes| {
            // Try to expand a `.`
            let dot = color_fallible_syntax_with(
                &ColorableDotShape,
                &FlatShape::Dot,
                token_nodes,
                context,
            );

            match dot {
                Ok(_) => {
                    // we found a dot, so let's keep looking for a member; if no member was found, fail
                    color_fallible_syntax(&MemberShape, token_nodes, context)?;

                    Ok(ContinuationInfo::Dot)
                }
                Err(_) => {
                    let result = token_nodes.atomic(|token_nodes| {
                        // we didn't find a dot, so let's see if we're looking at an infix. If not found, fail
                        color_fallible_syntax(&InfixShape, token_nodes, context)?;

                        // now that we've seen an infix shape, look for any expression. If not found, fail
                        color_fallible_syntax(&AnyExpressionShape, token_nodes, context)?;

                        Ok(ContinuationInfo::Infix)
                    })?;

                    Ok(result)
                }
            }
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct VariableShape;

impl ExpandExpression for VariableShape {
    fn name(&self) -> &'static str {
        "variable"
    }

    fn expand_expr<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<hir::Expression, ParseError> {
        parse_single_node(token_nodes, "variable", |token, token_tag, err| {
            Ok(match token {
                UnspannedToken::Variable(tag) => {
                    if tag.slice(context.source) == "it" {
                        hir::Expression::it_variable(tag, token_tag)
                    } else {
                        hir::Expression::variable(tag, token_tag)
                    }
                }
                _ => return Err(err.error()),
            })
        })
    }
}

impl FallibleColorSyntax for VariableShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "VariableShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let atom = expand_atom(
            token_nodes,
            "variable",
            context,
            ExpansionRule::permissive(),
        );

        let atom = match atom {
            Err(err) => return Err(err.into()),
            Ok(atom) => atom,
        };

        match &atom.unspanned {
            UnspannedAtomicToken::Variable { .. } => {
                token_nodes.color_shape(FlatShape::Variable.spanned(atom.span));
                Ok(())
            }
            UnspannedAtomicToken::ItVariable { .. } => {
                token_nodes.color_shape(FlatShape::ItVariable.spanned(atom.span));
                Ok(())
            }
            _ => Err(ParseError::mismatch("variable", atom.type_name().spanned(atom.span)).into()),
        }
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
    pub fn to_expr(&self) -> hir::Expression {
        match self {
            Member::String(outer, inner) => hir::Expression::string(*inner, *outer),
            Member::Int(number, span) => hir::Expression::number(number.clone(), *span),
            Member::Bare(span) => hir::Expression::string(*span, *span),
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

    pub fn into_path(self, next: Peeked) -> Result<Tagged<Vec<Member>>, ParseError> {
        match self {
            ColumnPathState::Initial => Err(next.type_error("column path")),
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

pub fn expand_column_path<'a, 'b>(
    token_nodes: &'b mut TokensIterator<'a>,
    context: &ExpandContext,
) -> Result<ColumnPathSyntax, ParseError> {
    let mut state = ColumnPathState::Initial;

    loop {
        let member = expand_syntax(&MemberShape, token_nodes, context);

        match member {
            Err(_) => break,
            Ok(member) => state = state.member(member),
        }

        let dot = expand_syntax(&DotShape, token_nodes, context);

        match dot {
            Err(_) => break,
            Ok(dot) => state = state.dot(dot),
        }
    }

    let path = state.into_path(token_nodes.peek_non_ws())?;

    Ok(ColumnPathSyntax {
        path: path.item,
        tag: path.tag,
    })
}

#[derive(Debug, Copy, Clone)]
pub struct ColumnPathShape;

impl FallibleColorSyntax for ColumnPathShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "ColumnPathShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        // If there's not even one member shape, fail
        color_fallible_syntax(&MemberShape, token_nodes, context)?;

        loop {
            let checkpoint = token_nodes.checkpoint();

            match color_fallible_syntax_with(
                &ColorableDotShape,
                &FlatShape::Dot,
                checkpoint.iterator,
                context,
            ) {
                Err(_) => {
                    // we already saw at least one member shape, so return successfully
                    return Ok(());
                }

                Ok(_) => {
                    match color_fallible_syntax(&MemberShape, checkpoint.iterator, context) {
                        Err(_) => {
                            // we saw a dot but not a member (but we saw at least one member),
                            // so don't commit the dot but return successfully
                            return Ok(());
                        }

                        Ok(_) => {
                            // we saw a dot and a member, so commit it and continue on
                            checkpoint.commit();
                        }
                    }
                }
            }
        }
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

impl ExpandSyntax for ColumnPathShape {
    type Output = ColumnPathSyntax;

    fn name(&self) -> &'static str {
        "column path"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        expand_column_path(token_nodes, context)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct MemberShape;

impl FallibleColorSyntax for MemberShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "MemberShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let bare =
            color_fallible_syntax_with(&BareShape, &FlatShape::BareMember, token_nodes, context);

        if bare.is_ok() {
            return Ok(());
        }

        // If we don't have a bare word, we'll look for a string

        // Look for a string token. If we don't find one, fail
        color_fallible_syntax_with(&StringShape, &FlatShape::StringMember, token_nodes, context)
    }
}

#[derive(Debug, Copy, Clone)]
struct IntMemberShape;

impl ExpandSyntax for IntMemberShape {
    type Output = Member;

    fn name(&self) -> &'static str {
        "integer member"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let next = expand_atom(
                token_nodes,
                "integer member",
                context,
                ExpansionRule::new().separate_members(),
            )?;

            match next.unspanned {
                UnspannedAtomicToken::Number {
                    number: RawNumber::Int(int),
                } => Ok(Member::Int(
                    BigInt::from_str(int.slice(context.source)).map_err(|_| {
                        ParseError::internal_error(
                            "can't convert from string to big int".spanned(int),
                        )
                    })?,
                    int,
                )),

                UnspannedAtomicToken::Word { text } => {
                    let int = BigInt::from_str(text.slice(context.source));

                    match int {
                        Ok(int) => Ok(Member::Int(int, text)),
                        Err(_) => Err(ParseError::mismatch("integer member", "word".spanned(text))),
                    }
                }

                other => Err(ParseError::mismatch(
                    "integer member",
                    other.type_name().spanned(next.span),
                )),
            }
        })
    }
}

impl ExpandSyntax for MemberShape {
    type Output = Member;

    fn name(&self) -> &'static str {
        "column"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<Member, ParseError> {
        if let Ok(int) = expand_syntax(&IntMemberShape, token_nodes, context) {
            return Ok(int);
        }

        let bare = BareShape.test(token_nodes, context);
        if let Some(peeked) = bare {
            let node = peeked.not_eof("column")?.commit();
            return Ok(Member::Bare(node.span()));
        }

        /* KATZ */
        /* let number = NumberShape.test(token_nodes, context);

        if let Some(peeked) = number {
            let node = peeked.not_eof("column")?.commit();
            let (n, span) = node.as_number().ok_or_else(|| {
                ParseError::internal_error("can't convert node to number".spanned(node.span()))
            })?;

            return Ok(Member::Number(n, span))
        }*/

        let string = StringShape.test(token_nodes, context);

        if let Some(peeked) = string {
            let node = peeked.not_eof("column")?.commit();
            let (outer, inner) = node.as_string().ok_or_else(|| {
                ParseError::internal_error("can't convert node to string".spanned(node.span()))
            })?;

            return Ok(Member::String(outer, inner));
        }

        Err(token_nodes.peek_any().type_error("column"))
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DotShape;

#[derive(Debug, Copy, Clone)]
pub struct ColorableDotShape;

impl FallibleColorSyntax for ColorableDotShape {
    type Info = ();
    type Input = FlatShape;

    fn name(&self) -> &'static str {
        "ColorableDotShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        input: &FlatShape,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let peeked = token_nodes.peek_any().not_eof("dot")?;

        match peeked.node {
            node if node.is_dot() => {
                peeked.commit();
                token_nodes.color_shape((*input).spanned(node.span()));
                Ok(())
            }

            other => Err(ShellError::type_error(
                "dot",
                other.type_name().spanned(other.span()),
            )),
        }
    }
}

impl SkipSyntax for DotShape {
    fn skip<'a, 'b>(
        &self,
        token_nodes: &mut TokensIterator<'_>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        expand_syntax(self, token_nodes, context)?;

        Ok(())
    }
}

impl ExpandSyntax for DotShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "dot"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        parse_single_node(token_nodes, "dot", |token, token_span, _| {
            Ok(match token {
                UnspannedToken::EvaluationOperator(EvaluationOperator::Dot) => token_span,
                _ => {
                    return Err(ParseError::mismatch(
                        "dot",
                        token.type_name().spanned(token_span),
                    ))
                }
            })
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InfixShape;

impl FallibleColorSyntax for InfixShape {
    type Info = ();
    type Input = ();

    fn name(&self) -> &'static str {
        "InfixShape"
    }

    fn color_syntax<'a, 'b>(
        &self,
        _input: &(),
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<(), ShellError> {
        let checkpoint = token_nodes.checkpoint();

        // An infix operator must be prefixed by whitespace. If no whitespace was found, fail
        color_fallible_syntax(&WhitespaceShape, checkpoint.iterator, context)?;

        // Parse the next TokenNode after the whitespace
        let operator_span = parse_single_node(
            checkpoint.iterator,
            "infix operator",
            |token, token_span, _| {
                match token {
                    // If it's an operator (and not `.`), it's a match
                    UnspannedToken::CompareOperator(_operator) => Ok(token_span),

                    // Otherwise, it's not a match
                    _ => Err(ParseError::mismatch(
                        "infix operator",
                        token.type_name().spanned(token_span),
                    )),
                }
            },
        )?;

        checkpoint
            .iterator
            .color_shape(FlatShape::CompareOperator.spanned(operator_span));

        // An infix operator must be followed by whitespace. If no whitespace was found, fail
        color_fallible_syntax(&WhitespaceShape, checkpoint.iterator, context)?;

        checkpoint.commit();
        Ok(())
    }
}

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
    type Output = InfixSyntax;

    fn name(&self) -> &'static str {
        "infix operator"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let mut checkpoint = token_nodes.checkpoint();

        // An infix operator must be prefixed by whitespace
        let start = expand_syntax(&WhitespaceShape, checkpoint.iterator, context)?;

        // Parse the next TokenNode after the whitespace
        let operator = expand_syntax(&InfixInnerShape, &mut checkpoint.iterator, context)?;

        // An infix operator must be followed by whitespace
        let end = expand_syntax(&WhitespaceShape, checkpoint.iterator, context)?;

        checkpoint.commit();

        Ok(InfixSyntax {
            infix: (start, operator, end).spanned(start.until(end)),
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
    type Output = InfixInnerSyntax;

    fn name(&self) -> &'static str {
        "infix inner"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        _context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        parse_single_node(token_nodes, "infix operator", |token, token_span, err| {
            Ok(match token {
                // If it's a comparison operator, it's a match
                UnspannedToken::CompareOperator(operator) => InfixInnerSyntax {
                    operator: operator.spanned(token_span),
                },

                // Otherwise, it's not a match
                _ => return Err(err.error()),
            })
        })
    }
}
