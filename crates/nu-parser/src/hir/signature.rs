use crate::hir;
use crate::hir::syntax_shape::{
    expand_atom, expand_syntax, BareShape, ExpandContext, ExpandSyntax, ExpansionRule,
    UnspannedAtomicToken, WhitespaceShape,
};
use crate::hir::tokens_iterator::TokensIterator;
use crate::parse::comment::Comment;
use derive_new::new;
use nu_errors::ParseError;
use nu_protocol::{RowType, SpannedTypeName, Type};
use nu_source::{
    b, DebugDocBuilder, HasFallibleSpan, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem,
};
use std::fmt::Debug;

// A Signature is a command without implementation.
//
// In Nu, a command is a function combined with macro expansion rules.
//
// def cd
//   # Change to a new path.
//   optional directory(Path) # the directory to change to
// end

#[derive(new)]
struct Expander<'a, 'b, 'c, 'd> {
    iterator: &'b mut TokensIterator<'a>,
    context: &'d ExpandContext<'c>,
}

impl<'a, 'b, 'c, 'd> Expander<'a, 'b, 'c, 'd> {
    fn expand<O>(&mut self, syntax: impl ExpandSyntax<Output = O>) -> Result<O, ParseError>
    where
        O: HasFallibleSpan + Clone + std::fmt::Debug + 'static,
    {
        expand_syntax(&syntax, self.iterator, self.context)
    }

    fn optional<O>(&mut self, syntax: impl ExpandSyntax<Output = O>) -> Option<O>
    where
        O: HasFallibleSpan + Clone + std::fmt::Debug + 'static,
    {
        match expand_syntax(&syntax, self.iterator, self.context) {
            Err(_) => None,
            Ok(value) => Some(value),
        }
    }

    fn pos(&mut self) -> Span {
        self.iterator.span_at_cursor()
    }

    fn slice_string(&mut self, span: impl Into<Span>) -> String {
        span.into().slice(self.context.source()).to_string()
    }
}

#[derive(Debug, Copy, Clone)]
struct SignatureShape;

impl ExpandSyntax for SignatureShape {
    type Output = hir::Signature;

    fn name(&self) -> &'static str {
        "signature"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let mut expander = Expander::new(token_nodes, context);
            let start = expander.pos();
            expander.expand(keyword("def"))?;
            expander.expand(WhitespaceShape)?;
            let name = expander.expand(BareShape)?;
            expander.expand(SeparatorShape)?;
            let usage = expander.expand(CommentShape)?;
            expander.expand(SeparatorShape)?;
            let end = expander.pos();

            Ok(hir::Signature::new(
                nu_protocol::Signature::new(&name.word).desc(expander.slice_string(usage.text)),
                start.until(end),
            ))
        })
    }
}

fn keyword(kw: &'static str) -> KeywordShape {
    KeywordShape { keyword: kw }
}

#[derive(Debug, Copy, Clone)]
struct KeywordShape {
    keyword: &'static str,
}

impl ExpandSyntax for KeywordShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "keyword"
    }
    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(token_nodes, "keyword", context, ExpansionRule::new())?;

        if let UnspannedAtomicToken::Word { text } = &atom.unspanned {
            let word = text.slice(context.source());

            if word == self.keyword {
                return Ok(atom.span);
            }
        }

        Err(ParseError::mismatch(self.keyword, atom.spanned_type_name()))
    }
}

#[derive(Debug, Copy, Clone)]
struct SeparatorShape;

impl ExpandSyntax for SeparatorShape {
    type Output = Span;

    fn name(&self) -> &'static str {
        "separator"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(token_nodes, "separator", context, ExpansionRule::new())?;

        match &atom.unspanned {
            UnspannedAtomicToken::Separator { text } => Ok(*text),
            _ => Err(ParseError::mismatch("separator", atom.spanned_type_name())),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct CommentShape;

impl ExpandSyntax for CommentShape {
    type Output = Comment;

    fn name(&self) -> &'static str {
        "comment"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(token_nodes, "comment", context, ExpansionRule::new())?;

        match &atom.unspanned {
            UnspannedAtomicToken::Comment { body } => Ok(Comment::line(body, atom.span)),
            _ => Err(ParseError::mismatch("separator", atom.spanned_type_name())),
        }
    }
}

#[derive(Debug, Copy, Clone, new)]
struct TupleShape<A, B> {
    first: A,
    second: B,
}

#[derive(Debug, Clone, new)]
struct TupleSyntax<A, B> {
    first: A,
    second: B,
}

impl<A, B> PrettyDebugWithSource for TupleSyntax<A, B>
where
    A: PrettyDebugWithSource,
    B: PrettyDebugWithSource,
{
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "pair",
            self.first.pretty_debug(source) + b::space() + self.second.pretty_debug(source),
        )
    }
}

impl<A, B> HasFallibleSpan for TupleSyntax<A, B>
where
    A: HasFallibleSpan + Debug + Clone,
    B: HasFallibleSpan + Debug + Clone,
{
    fn maybe_span(&self) -> Option<Span> {
        match (self.first.maybe_span(), self.second.maybe_span()) {
            (Some(first), Some(second)) => Some(first.until(second)),
            (Some(first), None) => Some(first),
            (None, Some(second)) => Some(second),
            (None, None) => None,
        }
    }
}

impl<A, B, AOut, BOut> ExpandSyntax for TupleShape<A, B>
where
    A: ExpandSyntax<Output = AOut> + Debug + Copy,
    B: ExpandSyntax<Output = BOut> + Debug + Copy,
    AOut: HasFallibleSpan + Debug + Clone + 'static,
    BOut: HasFallibleSpan + Debug + Clone + 'static,
{
    type Output = TupleSyntax<AOut, BOut>;

    fn name(&self) -> &'static str {
        "pair"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let first = expand_syntax(&self.first, token_nodes, context)?;
            let second = expand_syntax(&self.second, token_nodes, context)?;

            Ok(TupleSyntax { first, second })
        })
    }
}

#[derive(Debug, Clone)]
pub struct PositionalParam {
    optional: Option<Span>,
    name: Identifier,
    ty: Spanned<Type>,
    desc: Spanned<String>,
    span: Span,
}

impl HasSpan for PositionalParam {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for PositionalParam {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        (match self.optional {
            Some(_) => b::description("optional") + b::space(),
            None => b::blank(),
        }) + self.ty.pretty_debug(source)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PositionalParamShape;

impl ExpandSyntax for PositionalParamShape {
    type Output = PositionalParam;

    fn name(&self) -> &'static str {
        "positional param"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        token_nodes.atomic_parse(|token_nodes| {
            let mut expander = Expander::new(token_nodes, context);

            let optional = expander
                .optional(TupleShape::new(keyword("optional"), WhitespaceShape))
                .map(|s| s.first);

            let name = expander.expand(IdentifierShape)?;

            expander.optional(WhitespaceShape);

            let _ty = expander.expand(TypeShape)?;

            Ok(PositionalParam {
                optional,
                name,
                ty: Type::Nothing.spanned(Span::unknown()),
                desc: format!("").spanned(Span::unknown()),
                span: Span::unknown(),
            })
        })
    }
}

#[derive(Debug, Clone)]
struct Identifier {
    body: String,
    span: Span,
}

impl HasSpan for Identifier {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for Identifier {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed("id", b::description(self.span.slice(source)))
    }
}

#[derive(Debug, Copy, Clone)]
struct IdentifierShape;

impl ExpandSyntax for IdentifierShape {
    type Output = Identifier;
    fn name(&self) -> &'static str {
        "identifier"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(token_nodes, "identifier", context, ExpansionRule::new())?;

        if let UnspannedAtomicToken::Word { text } = atom.unspanned {
            let body = text.slice(context.source());
            if is_id(body) {
                return Ok(Identifier {
                    body: body.to_string(),
                    span: text,
                });
            }
        }

        Err(ParseError::mismatch("identifier", atom.spanned_type_name()))
    }
}

fn is_id(input: &str) -> bool {
    let source = nu_source::nom_input(input);
    match crate::parse::parser::ident(source) {
        Err(_) => false,
        Ok((input, _)) => input.fragment.is_empty(),
    }
}

#[derive(Debug, Clone, new)]
struct TypeSyntax {
    ty: Type,
    span: Span,
}

impl HasSpan for TypeSyntax {
    fn span(&self) -> Span {
        self.span
    }
}

impl PrettyDebugWithSource for TypeSyntax {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        self.ty.pretty_debug(source)
    }
}

#[derive(Debug, Copy, Clone)]
struct TypeShape;

impl ExpandSyntax for TypeShape {
    type Output = TypeSyntax;

    fn name(&self) -> &'static str {
        "type"
    }
    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(token_nodes, "type", context, ExpansionRule::new())?;

        match atom.unspanned {
            UnspannedAtomicToken::Word { text } => {
                let word = text.slice(context.source());

                Ok(TypeSyntax::new(
                    match word {
                        "nothing" => Type::Nothing,
                        "integer" => Type::Int,
                        "decimal" => Type::Decimal,
                        "bytesize" => Type::Bytesize,
                        "string" => Type::String,
                        "column-path" => Type::ColumnPath,
                        "pattern" => Type::Pattern,
                        "boolean" => Type::Boolean,
                        "date" => Type::Date,
                        "duration" => Type::Duration,
                        "filename" => Type::Path,
                        "binary" => Type::Binary,
                        "row" => Type::Row(RowType::new()),
                        "table" => Type::Table(vec![]),
                        "block" => Type::Block,
                        _ => return Err(ParseError::mismatch("type", atom.spanned_type_name())),
                    },
                    atom.span,
                ))
            }
            _ => Err(ParseError::mismatch("type", atom.spanned_type_name())),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct TypeAnnotation;

impl ExpandSyntax for TypeAnnotation {
    type Output = TypeSyntax;

    fn name(&self) -> &'static str {
        "type annotation"
    }

    fn expand_syntax<'a, 'b>(
        &self,
        token_nodes: &'b mut TokensIterator<'a>,
        context: &ExpandContext,
    ) -> Result<Self::Output, ParseError> {
        let atom = expand_atom(
            token_nodes,
            "type annotation",
            context,
            ExpansionRule::new(),
        )?;

        match atom.unspanned {
            UnspannedAtomicToken::RoundDelimited { nodes, .. } => {
                token_nodes.atomic_parse(|token_nodes| {
                    token_nodes.child(
                        (&nodes[..]).spanned(atom.span),
                        context.source().clone(),
                        |token_nodes| {
                            let ty = expand_syntax(&TypeShape, token_nodes, context)?;

                            let next = token_nodes.peek_non_ws();

                            match next.node {
                                None => Ok(ty),
                                Some(node) => {
                                    Err(ParseError::extra_tokens(node.spanned_type_name()))
                                }
                            }
                        },
                    )
                })
            }

            _ => Err(ParseError::mismatch(
                "type annotation",
                atom.spanned_type_name(),
            )),
        }
    }
}
