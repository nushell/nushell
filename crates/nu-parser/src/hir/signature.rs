// use crate::hir;
// use crate::hir::syntax_shape::{
//     expand_atom, expand_syntax, BareShape, ExpandContext, ExpandSyntax, ExpansionRule, FlatShape,
//     UnspannedAtomicToken, WhitespaceShape,
// };
// use crate::hir::tokens_iterator::TokensIterator;
// use crate::parse::comment::Comment;
// use crate::parse::token_tree::{DelimitedNode, Delimiter, UnspannedTokenNode};
// use derive_new::new;
// use nu_errors::ParseError;
// use nu_protocol::{RowType, SpannedTypeName, Type};
// use nu_source::{
//     b, DebugDocBuilder, HasFallibleSpan, HasSpan, PrettyDebugWithSource, Span, Spanned, SpannedItem,
// };
// use std::fmt::Debug;

// A Signature is a command without implementation.
//
// In Nu, a command is a function combined with macro expansion rules.
//
// def cd
//   # Change to a new path.
//   optional directory(Path) # the directory to change to
// end

// #[derive(new)]
// struct Expander<'a, 'b, 'c, 'd> {
//     iterator: &'b mut TokensIterator<'a>,
//     context: &'d ExpandContext<'c>,
// }

// impl<'a, 'b, 'c, 'd> Expander<'a, 'b, 'c, 'd> {
//     fn expand<O>(&mut self, syntax: impl ExpandSyntax<Output = O>) -> O
//     where
//         O: HasFallibleSpan + Clone + std::fmt::Debug + 'static,
//     {
//         expand_syntax(&syntax, self.iterator, self.context)
//     }

//     fn optional<O>(
//         &mut self,
//         syntax: impl ExpandSyntax<Output = Result<O, ParseError>>,
//     ) -> Option<O>
//     where
//         O: HasFallibleSpan + Clone + std::fmt::Debug + 'static,
//     {
//         match expand_syntax(&syntax, self.iterator, self.context) {
//             Err(_) => None,
//             Ok(value) => Some(value),
//         }
//     }

//     fn pos(&mut self) -> Span {
//         self.iterator.span_at_cursor()
//     }

//     fn slice_string(&mut self, span: impl Into<Span>) -> String {
//         span.into().slice(self.context.source()).to_string()
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct SignatureShape;

// impl ExpandSyntax for SignatureShape {
//     type Output = Result<hir::Signature, ParseError>;

//     fn name(&self) -> &'static str {
//         "signature"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<hir::Signature, ParseError> {
//         token_nodes.atomic_parse(|token_nodes| {
//             let mut expander = Expander::new(token_nodes, context);
//             let start = expander.pos();
//             expander.expand(keyword("def"))?;
//             expander.expand(WhitespaceShape);
//             let name = expander.expand(BareShape)?;
//             expander.expand(SeparatorShape)?;
//             let usage = expander.expand(CommentShape)?;
//             expander.expand(SeparatorShape)?;
//             let end = expander.pos();

//             Ok(hir::Signature::new(
//                 nu_protocol::Signature::new(&name.word).desc(expander.slice_string(usage.text)),
//                 start.until(end),
//             ))
//         })
//     }
// }

// fn keyword(kw: &'static str) -> KeywordShape {
//     KeywordShape { keyword: kw }
// }

// #[derive(Debug, Copy, Clone)]
// struct KeywordShape {
//     keyword: &'static str,
// }

// impl ExpandSyntax for KeywordShape {
//     type Output = Result<Span, ParseError>;

//     fn name(&self) -> &'static str {
//         "keyword"
//     }
//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<Span, ParseError> {
//         token_nodes.color_any_syntax("keyword", |token, err| match token.unspanned() {
//             UnspannedTokenNode::Bare => {
//                 let span = token.span();
//                 let word = span.slice(context.source());

//                 if word == self.keyword {
//                     Ok((FlatShape::Keyword, span))
//                 } else {
//                     err()
//                 }
//             }
//             _ => err(),
//         })
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct SeparatorShape;

// impl ExpandSyntax for SeparatorShape {
//     type Output = Result<Span, ParseError>;

//     fn name(&self) -> &'static str {
//         "separator"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         _context: &ExpandContext,
//     ) -> Result<Span, ParseError> {
//         token_nodes.color_any_syntax("separator", |token, err| match token.unspanned() {
//             UnspannedTokenNode::Separator => Ok((FlatShape::Separator, token.span())),
//             _ => err(),
//         })
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct CommentShape;

// impl ExpandSyntax for CommentShape {
//     type Output = Result<Comment, ParseError>;

//     fn name(&self) -> &'static str {
//         "comment"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         _context: &ExpandContext,
//     ) -> Result<Comment, ParseError> {
//         token_nodes.color_any_syntax("comment", |token, err| match token.unspanned() {
//             UnspannedTokenNode::Comment(comment) => Ok((
//                 FlatShape::Comment,
//                 Comment::line(comment.text, token.span()),
//             )),
//             _ => err(),
//         })
//     }
// }

// #[derive(Debug, Copy, Clone, new)]
// struct TupleShape<A, B> {
//     first: A,
//     second: B,
// }

// #[derive(Debug, Clone, new)]
// struct TupleSyntax<A, B> {
//     first: A,
//     second: B,
// }

// impl<A, B> PrettyDebugWithSource for TupleSyntax<A, B>
// where
//     A: PrettyDebugWithSource,
//     B: PrettyDebugWithSource,
// {
//     fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
//         b::typed(
//             "pair",
//             self.first.pretty_debug(source) + b::space() + self.second.pretty_debug(source),
//         )
//     }
// }

// impl<A, B> HasFallibleSpan for TupleSyntax<A, B>
// where
//     A: HasFallibleSpan + Debug + Clone,
//     B: HasFallibleSpan + Debug + Clone,
// {
//     fn maybe_span(&self) -> Option<Span> {
//         match (self.first.maybe_span(), self.second.maybe_span()) {
//             (Some(first), Some(second)) => Some(first.until(second)),
//             (Some(first), None) => Some(first),
//             (None, Some(second)) => Some(second),
//             (None, None) => None,
//         }
//     }
// }

// impl<A, B, AOut, BOut> ExpandSyntax for TupleShape<A, B>
// where
//     A: ExpandSyntax<Output = Result<AOut, ParseError>> + Debug + Copy,
//     B: ExpandSyntax<Output = Result<BOut, ParseError>> + Debug + Copy,
//     AOut: HasFallibleSpan + Debug + Clone + 'static,
//     BOut: HasFallibleSpan + Debug + Clone + 'static,
// {
//     type Output = Result<TupleSyntax<AOut, BOut>, ParseError>;

//     fn name(&self) -> &'static str {
//         "pair"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<TupleSyntax<AOut, BOut>, ParseError> {
//         token_nodes.atomic_parse(|token_nodes| {
//             let first = expand_syntax(&self.first, token_nodes, context)?;
//             let second = expand_syntax(&self.second, token_nodes, context)?;

//             Ok(TupleSyntax { first, second })
//         })
//     }
// }

// #[derive(Debug, Clone)]
// pub struct PositionalParam {
//     optional: Option<Span>,
//     name: Identifier,
//     ty: Spanned<Type>,
//     desc: Spanned<String>,
//     span: Span,
// }

// impl HasSpan for PositionalParam {
//     fn span(&self) -> Span {
//         self.span
//     }
// }

// impl PrettyDebugWithSource for PositionalParam {
//     fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
//         (match self.optional {
//             Some(_) => b::description("optional") + b::space(),
//             None => b::blank(),
//         }) + self.ty.pretty_debug(source)
//     }
// }

// #[derive(Debug, Copy, Clone)]
// pub struct PositionalParamShape;

// impl ExpandSyntax for PositionalParamShape {
//     type Output = Result<PositionalParam, ParseError>;

//     fn name(&self) -> &'static str {
//         "positional param"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<PositionalParam, ParseError> {
//         token_nodes.atomic_parse(|token_nodes| {
//             let mut expander = Expander::new(token_nodes, context);

//             let optional = expander
//                 .optional(TupleShape::new(keyword("optional"), WhitespaceShape))
//                 .map(|s| s.first);

//             let name = expander.expand(IdentifierShape)?;

//             expander.optional(WhitespaceShape);

//             let _ty = expander.expand(TypeShape)?;

//             Ok(PositionalParam {
//                 optional,
//                 name,
//                 ty: Type::Nothing.spanned(Span::unknown()),
//                 desc: format!("").spanned(Span::unknown()),
//                 span: Span::unknown(),
//             })
//         })
//     }
// }

// fn fallible<T, U>(shape: T) -> FallibleShape<T, U>
// where
//     T: ExpandSyntax<Output = U>,
//     U: Clone + std::fmt::Debug + 'static,
// {
//     FallibleShape { inner: shape }
// }

// #[derive(Debug, Clone)]
// struct Identifier {
//     body: String,
//     span: Span,
// }

// impl HasSpan for Identifier {
//     fn span(&self) -> Span {
//         self.span
//     }
// }

// impl PrettyDebugWithSource for Identifier {
//     fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
//         b::typed("id", b::description(self.span.slice(source)))
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct IdentifierShape;

// impl ExpandSyntax for IdentifierShape {
//     type Output = Result<Identifier, ParseError>;

//     fn name(&self) -> &'static str {
//         "identifier"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<Identifier, ParseError> {
//         token_nodes.color_any_syntax("identifier", |token, err| match token.unspanned() {
//             UnspannedTokenNode::Bare => {
//                 let body = token.span().slice(context.source());

//                 if is_id(body) {
//                     return Ok((
//                         FlatShape::Identifier,
//                         Identifier {
//                             body: body.to_string(),
//                             span: token.span(),
//                         },
//                     ));
//                 } else {
//                     err()
//                 }
//             }

//             _ => err(),
//         })
//     }
// }

// fn is_id(input: &str) -> bool {
//     let source = nu_source::nom_input(input);
//     match crate::parse::parser::ident(source) {
//         Err(_) => false,
//         Ok((input, _)) => input.fragment.len() == 0,
//     }
// }

// #[derive(Debug, Clone, new)]
// struct TypeSyntax {
//     ty: Type,
//     span: Span,
// }

// impl HasSpan for TypeSyntax {
//     fn span(&self) -> Span {
//         self.span
//     }
// }

// impl PrettyDebugWithSource for TypeSyntax {
//     fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
//         self.ty.pretty_debug(source)
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct TypeShape;

// impl ExpandSyntax for TypeShape {
//     type Output = Result<TypeSyntax, ParseError>;

//     fn name(&self) -> &'static str {
//         "type"
//     }
//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         context: &ExpandContext,
//     ) -> Result<TypeSyntax, ParseError> {
//         token_nodes.color_any_syntax("type", |token, err| match token.unspanned() {
//             UnspannedTokenNode::Bare => {
//                 let word = token.span().slice(context.source);

//                 Ok((
//                     FlatShape::Type,
//                     TypeSyntax::new(
//                         match word {
//                             "nothing" => Type::Nothing,
//                             "integer" => Type::Int,
//                             "decimal" => Type::Decimal,
//                             "bytesize" => Type::Bytesize,
//                             "string" => Type::String,
//                             "column-path" => Type::ColumnPath,
//                             "pattern" => Type::Pattern,
//                             "boolean" => Type::Boolean,
//                             "date" => Type::Date,
//                             "duration" => Type::Duration,
//                             "filename" => Type::Path,
//                             "binary" => Type::Binary,
//                             "row" => Type::Row(RowType::new()),
//                             "table" => Type::Table(vec![]),
//                             "block" => Type::Block,
//                             _ => return err(),
//                         },
//                         token.span(),
//                     ),
//                 ))
//             }

//             _ => err(),
//         })?;

//         let atom = expand_atom(token_nodes, "type", context, ExpansionRule::new())?;

//         match atom.unspanned {
//             UnspannedAtomicToken::Word { text } => {
//                 let word = text.slice(context.source());

//                 Ok(TypeSyntax::new(
//                     match word {
//                         "nothing" => Type::Nothing,
//                         "integer" => Type::Int,
//                         "decimal" => Type::Decimal,
//                         "bytesize" => Type::Bytesize,
//                         "string" => Type::String,
//                         "column-path" => Type::ColumnPath,
//                         "pattern" => Type::Pattern,
//                         "boolean" => Type::Boolean,
//                         "date" => Type::Date,
//                         "duration" => Type::Duration,
//                         "filename" => Type::Path,
//                         "binary" => Type::Binary,
//                         "row" => Type::Row(RowType::new()),
//                         "table" => Type::Table(vec![]),
//                         "block" => Type::Block,
//                         _ => return Err(ParseError::mismatch("type", atom.spanned_type_name())),
//                     },
//                     atom.span,
//                 ))
//             }
//             _ => Err(ParseError::mismatch("type", atom.spanned_type_name())),
//         }
//     }
// }

// #[derive(Debug, Copy, Clone)]
// struct TypeAnnotation;

// impl ExpandSyntax for TypeAnnotation {
//     type Output = Result<TypeSyntax, ParseError>;

//     fn name(&self) -> &'static str {
//         "type annotation"
//     }

//     fn expand<'a, 'b>(
//         &self,
//         token_nodes: &'b mut TokensIterator<'a>,
//         _context: &ExpandContext,
//     ) -> Result<TypeSyntax, ParseError> {
//         let _paren = token_nodes.peek_any_token("type annotation", |token, err, _| match token
//             .unspanned()
//         {
//             UnspannedTokenNode::Delimited(DelimitedNode {
//                 delimiter: Delimiter::Paren,
//                 spans,
//                 children,
//             }) => Ok(children.spanned(spans.0.until(spans.1))),
//             _ => err(),
//         })?;

//         unimplemented!("TypeAnnotation#expand_syntax")
//     }
// }
