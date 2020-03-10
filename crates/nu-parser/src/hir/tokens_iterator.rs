pub(crate) mod debug;
pub(crate) mod into_shapes;
pub(crate) mod pattern;
pub(crate) mod state;

#[cfg(test)]
mod tests;

use self::debug::ExpandTracer;
use self::into_shapes::IntoShapes;
use self::state::{Peeked, TokensIteratorState};

use crate::hir::syntax_shape::flat_shape::{FlatShape, ShapeResult};
use crate::hir::syntax_shape::{ExpandContext, ExpandSyntax, ExpressionListShape};
use crate::hir::SpannedExpression;
use crate::parse::token_tree::{BlockType, DelimitedNode, SpannedToken, SquareType, TokenType};

use getset::{Getters, MutGetters};
use nu_errors::ParseError;
use nu_protocol::SpannedTypeName;
use nu_source::{
    HasFallibleSpan, HasSpan, IntoSpanned, PrettyDebugWithSource, Span, Spanned, SpannedItem, Text,
};
use std::borrow::Borrow;
use std::sync::Arc;

#[derive(Getters, MutGetters, Clone, Debug)]
pub struct TokensIterator<'content> {
    #[get = "pub"]
    #[get_mut = "pub"]
    state: TokensIteratorState<'content>,
    #[get = "pub"]
    #[get_mut = "pub"]
    expand_tracer: ExpandTracer<SpannedExpression>,
}

#[derive(Debug)]
pub struct Checkpoint<'content, 'me> {
    pub(crate) iterator: &'me mut TokensIterator<'content>,
    index: usize,
    seen: indexmap::IndexSet<usize>,

    shape_start: usize,
    committed: bool,
}

impl<'content, 'me> Checkpoint<'content, 'me> {
    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}

impl<'content, 'me> std::ops::Drop for Checkpoint<'content, 'me> {
    fn drop(&mut self) {
        if !self.committed {
            let state = &mut self.iterator.state;

            state.index = self.index;
            state.seen = self.seen.clone();

            state.shapes.truncate(self.shape_start);
        }
    }
}

// For parse_command
impl<'content> TokensIterator<'content> {
    pub fn sort_shapes(&mut self) {
        // This is pretty dubious, but it works. We should look into a better algorithm that doesn't end up requiring
        // this solution.

        self.state
            .shapes
            .sort_by(|a, b| a.span().start().cmp(&b.span().start()));
    }

    /// Run a block of code, retrieving the shapes that were created during the block. This is
    /// used by `parse_command` to associate shapes with a particular flag.
    pub fn shapes_for<'me, T>(
        &'me mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, ParseError>,
    ) -> (Result<T, ParseError>, Vec<ShapeResult>) {
        let index = self.state.index;
        let mut shapes = vec![];
        let mut errors = self.state.errors.clone();

        let seen = self.state.seen.clone();
        std::mem::swap(&mut self.state.shapes, &mut shapes);
        std::mem::swap(&mut self.state.errors, &mut errors);

        let checkpoint = Checkpoint {
            iterator: self,
            index,
            seen,
            committed: false,
            shape_start: 0,
        };

        let value = block(checkpoint.iterator);

        let value = match value {
            Err(err) => {
                drop(checkpoint);
                std::mem::swap(&mut self.state.shapes, &mut shapes);
                std::mem::swap(&mut self.state.errors, &mut errors);
                return (Err(err), vec![]);
            }

            Ok(value) => value,
        };

        checkpoint.commit();
        std::mem::swap(&mut self.state.shapes, &mut shapes);

        (Ok(value), shapes)
    }

    pub fn extract<T>(&mut self, f: impl Fn(&SpannedToken) -> Option<T>) -> Option<(usize, T)> {
        let state = &mut self.state;

        for (i, item) in state.tokens.iter().enumerate() {
            if state.seen.contains(&i) {
                continue;
            }

            match f(item) {
                None => {
                    continue;
                }
                Some(value) => {
                    state.seen.insert(i);
                    return Some((i, value));
                }
            }
        }

        self.move_to(0);

        None
    }

    pub fn remove(&mut self, position: usize) {
        self.state.seen.insert(position);
    }
}

// Delimited
impl<'content> TokensIterator<'content> {
    pub fn block(&mut self) -> Result<Spanned<Vec<SpannedExpression>>, ParseError> {
        self.expand_token_with_token_nodes(BlockType, |node, token_nodes| {
            token_nodes.delimited(node)
        })
    }

    pub fn square(&mut self) -> Result<Spanned<Vec<SpannedExpression>>, ParseError> {
        self.expand_token_with_token_nodes(SquareType, |node, token_nodes| {
            token_nodes.delimited(node)
        })
    }

    fn delimited(
        &mut self,
        DelimitedNode {
            delimiter,
            spans,
            children,
        }: DelimitedNode,
    ) -> Result<(Vec<ShapeResult>, Spanned<Vec<SpannedExpression>>), ParseError> {
        let span = spans.0.until(spans.1);
        let (child_shapes, expr) = self.child(children[..].spanned(span), |token_nodes| {
            token_nodes.expand_infallible(ExpressionListShape).exprs
        });

        let mut shapes = vec![ShapeResult::Success(
            FlatShape::OpenDelimiter(delimiter).spanned(spans.0),
        )];
        shapes.extend(child_shapes);
        shapes.push(ShapeResult::Success(
            FlatShape::CloseDelimiter(delimiter).spanned(spans.1),
        ));

        Ok((shapes, expr))
    }
}

impl<'content> TokensIterator<'content> {
    pub fn new(
        items: &'content [SpannedToken],
        context: ExpandContext<'content>,
        span: Span,
    ) -> TokensIterator<'content> {
        let source = context.source();

        TokensIterator {
            state: TokensIteratorState {
                tokens: items,
                span,
                index: 0,
                seen: indexmap::IndexSet::new(),
                shapes: vec![],
                errors: indexmap::IndexMap::new(),
                context: Arc::new(context),
            },
            expand_tracer: ExpandTracer::new("Expand Trace", source.clone()),
        }
    }

    pub fn len(&self) -> usize {
        self.state.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.state.tokens.is_empty()
    }

    pub fn source(&self) -> Text {
        self.state.context.source().clone()
    }

    pub fn context(&self) -> &ExpandContext {
        &self.state.context
    }

    pub fn color_result(&mut self, shape: ShapeResult) {
        match shape {
            ShapeResult::Success(shape) => self.color_shape(shape),
            ShapeResult::Fallback { shape, allowed } => self.color_err(shape, allowed),
        }
    }

    pub fn color_shape(&mut self, shape: Spanned<FlatShape>) {
        self.with_tracer(|_, tracer| tracer.add_shape(shape.into_trace_shape(shape.span)));
        self.state.shapes.push(ShapeResult::Success(shape));
    }

    pub fn color_err(&mut self, shape: Spanned<FlatShape>, valid_shapes: Vec<String>) {
        self.with_tracer(|_, tracer| tracer.add_err_shape(shape.into_trace_shape(shape.span)));
        self.state.errors.insert(shape.span, valid_shapes.clone());
        self.state.shapes.push(ShapeResult::Fallback {
            shape,
            allowed: valid_shapes,
        });
    }

    pub fn color_shapes(&mut self, shapes: Vec<Spanned<FlatShape>>) {
        self.with_tracer(|_, tracer| {
            for shape in &shapes {
                tracer.add_shape(shape.into_trace_shape(shape.span))
            }
        });

        for shape in &shapes {
            self.state.shapes.push(ShapeResult::Success(*shape));
        }
    }

    pub fn child<'me, T>(
        &'me mut self,
        tokens: Spanned<&'me [SpannedToken]>,
        block: impl FnOnce(&mut TokensIterator<'me>) -> T,
    ) -> (Vec<ShapeResult>, T) {
        let mut shapes = vec![];
        std::mem::swap(&mut shapes, &mut self.state.shapes);

        let mut errors = self.state.errors.clone();
        std::mem::swap(&mut errors, &mut self.state.errors);

        let mut expand_tracer = ExpandTracer::new("Expand Trace", self.source());
        std::mem::swap(&mut expand_tracer, &mut self.expand_tracer);

        let mut iterator = TokensIterator {
            state: TokensIteratorState {
                tokens: tokens.item,
                span: tokens.span,
                index: 0,
                seen: indexmap::IndexSet::new(),
                shapes,
                errors,
                context: self.state.context.clone(),
            },
            expand_tracer,
        };

        let result = block(&mut iterator);

        std::mem::swap(&mut iterator.state.shapes, &mut self.state.shapes);
        std::mem::swap(&mut iterator.state.errors, &mut self.state.errors);
        std::mem::swap(&mut iterator.expand_tracer, &mut self.expand_tracer);

        (iterator.state.shapes, result)
    }

    fn with_tracer(
        &mut self,
        block: impl FnOnce(&mut TokensIteratorState, &mut ExpandTracer<SpannedExpression>),
    ) {
        let state = &mut self.state;
        let tracer = &mut self.expand_tracer;

        block(state, tracer)
    }

    pub fn finish_tracer(&mut self) {
        self.with_tracer(|_, tracer| tracer.finish())
    }

    pub fn atomic_parse<'me, T, E>(
        &'me mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, E>,
    ) -> Result<T, E> {
        let state = &mut self.state;

        let index = state.index;

        let shape_start = state.shapes.len();
        let seen = state.seen.clone();

        let checkpoint = Checkpoint {
            iterator: self,
            index,
            seen,
            committed: false,

            shape_start,
        };

        let value = block(checkpoint.iterator)?;

        checkpoint.commit();
        Ok(value)
    }

    fn eof_span(&self) -> Span {
        Span::new(self.state.span.end(), self.state.span.end())
    }

    pub fn span_at_cursor(&mut self) -> Span {
        let next = self.peek();

        match next.node {
            None => self.eof_span(),
            Some(node) => node.span(),
        }
    }

    pub fn at_end(&self) -> bool {
        next_index(&self.state).is_none()
    }

    pub fn move_to(&mut self, pos: usize) {
        self.state.index = pos;
    }

    /// Peek the next token in the token stream and return a `Peeked`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let peeked = token_nodes.peek().not_eof();
    /// let node = peeked.node;
    /// match node.unspanned() {
    ///     Token::Whitespace => {
    ///         let node = peeked.commit();
    ///         return Ok(node.span)
    ///     }
    ///     other => return Err(ParseError::mismatch("whitespace", node.spanned_type_name()))
    /// }
    /// ```
    pub fn peek<'me>(&'me mut self) -> Peeked<'content, 'me> {
        let state = self.state();
        let len = state.tokens.len();
        let from = state.index;

        let index = next_index(state);

        let (node, to) = match index {
            None => (None, len),

            Some(to) => (Some(&state.tokens[to]), to + 1),
        };

        Peeked {
            node,
            iterator: self,
            from,
            to,
        }
    }

    /// Produce an error corresponding to the next token.
    ///
    /// If the next token is EOF, produce an `UnexpectedEof`. Otherwise, produce a `Mismatch`.
    pub fn err_next_token(&mut self, expected: &'static str) -> ParseError {
        match next_index(&self.state) {
            None => ParseError::unexpected_eof(expected, self.eof_span()),
            Some(index) => {
                ParseError::mismatch(expected, self.state.tokens[index].spanned_type_name())
            }
        }
    }

    fn expand_token_with_token_nodes<
        'me,
        T: 'me,
        U: IntoSpanned<Output = V>,
        V: HasFallibleSpan,
        F: IntoShapes,
    >(
        &'me mut self,
        expected: impl TokenType<Output = T>,
        block: impl FnOnce(T, &mut Self) -> Result<(F, U), ParseError>,
    ) -> Result<V, ParseError> {
        let desc = expected.desc();

        let peeked = self.peek().not_eof(desc.borrow())?;

        let (shapes, val) = {
            let node = peeked.node;
            let type_name = node.spanned_type_name();

            let func = Box::new(|| Err(ParseError::mismatch(desc.clone().into_owned(), type_name)));

            match expected.extract_token_value(node, &func) {
                Err(err) => return Err(err),
                Ok(value) => match block(value, peeked.iterator) {
                    Err(err) => return Err(err),
                    Ok((shape, val)) => {
                        let span = peeked.node.span();
                        peeked.commit();
                        (shape.into_shapes(span), val.into_spanned(span))
                    }
                },
            }
        };

        for shape in &shapes {
            self.color_result(shape.clone());
        }

        Ok(val)
    }

    /// Expand and color a single token. Takes an `impl TokenType` and produces
    /// (() | FlatShape | Vec<Spanned<FlatShape>>, Output) (or an error).
    ///
    /// If a single FlatShape is produced, it is annotated with the span of the
    /// original token. Otherwise, each FlatShape in the list must already be
    /// annotated.
    pub fn expand_token<'me, T, U, V, F>(
        &'me mut self,
        expected: impl TokenType<Output = T>,
        block: impl FnOnce(T) -> Result<(F, U), ParseError>,
    ) -> Result<V, ParseError>
    where
        T: 'me,
        U: IntoSpanned<Output = V>,
        V: HasFallibleSpan,
        F: IntoShapes,
    {
        self.expand_token_with_token_nodes(expected, |value, _| block(value))
    }

    fn commit(&mut self, from: usize, to: usize) {
        for index in from..to {
            self.state.seen.insert(index);
        }

        self.state.index = to;
    }

    pub fn debug_remaining(&self) -> Vec<SpannedToken> {
        let mut tokens: TokensIterator = self.clone();
        tokens.move_to(0);
        tokens.cloned().collect()
    }

    /// Expand an `ExpandSyntax` whose output is a `Result`, producing either the shape's output
    /// or a `ParseError`. If the token stream is at EOF, this method produces a ParseError
    /// (`UnexpectedEof`).
    ///
    /// You must use `expand_syntax` if the `Output` of the `ExpandSyntax` is a `Result`, but
    /// it's difficult to model this in the Rust type system.
    pub fn expand_syntax<U>(
        &mut self,
        shape: impl ExpandSyntax<Output = Result<U, ParseError>>,
    ) -> Result<U, ParseError>
    where
        U: std::fmt::Debug + HasFallibleSpan + PrettyDebugWithSource + Clone + 'static,
    {
        if self.at_end() {
            self.with_tracer(|_, tracer| tracer.start(shape.name(), None));
            self.with_tracer(|_, tracer| tracer.eof_frame());
            return Err(ParseError::unexpected_eof(shape.name(), self.eof_span()));
        }

        let (result, added_shapes) = self.expand(shape);

        match &result {
            Ok(val) => self.finish_expand(val, added_shapes),
            Err(err) => self.with_tracer(|_, tracer| tracer.failed(err)),
        }

        result
    }

    /// Expand an `impl ExpandSyntax` and produce its Output. Use `expand_infallible` if the
    /// `ExpandSyntax` cannot produce a `Result`. You must also use `ExpandSyntax` if EOF
    /// is an error.
    ///
    /// The purpose of `expand_infallible` is to clearly mark the infallible path through
    /// and entire list of tokens that produces a fully colored version of the source.
    ///
    /// If the `ExpandSyntax` can poroduce a `Result`, make sure to use `expand_syntax`,
    /// which will correctly show the error in the trace.
    pub fn expand_infallible<U>(&mut self, shape: impl ExpandSyntax<Output = U>) -> U
    where
        U: std::fmt::Debug + PrettyDebugWithSource + HasFallibleSpan + Clone + 'static,
    {
        let (result, added_shapes) = self.expand(shape);

        self.finish_expand(&result, added_shapes);

        result
    }

    fn finish_expand<V>(&mut self, val: &V, added_shapes: usize)
    where
        V: PrettyDebugWithSource + HasFallibleSpan + Clone,
    {
        self.with_tracer(|_, tracer| {
            if val.maybe_span().is_some() || added_shapes > 0 {
                tracer.add_result(val.clone());
            }

            tracer.success();
        })
    }

    fn expand<U>(&mut self, shape: impl ExpandSyntax<Output = U>) -> (U, usize)
    where
        U: std::fmt::Debug + Clone + 'static,
    {
        let desc = shape.name();
        self.with_tracer(|state, tracer| {
            tracer.start(
                desc,
                next_index(state).map(|index| state.tokens[index].clone()),
            )
        });

        let start_shapes = self.state.shapes.len();
        let result = shape.expand(self);
        let added_shapes = self.state.shapes.len() - start_shapes;

        (result, added_shapes)
    }
}

impl<'content> Iterator for TokensIterator<'content> {
    type Item = &'content SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        next(self)
    }
}

fn next_index(state: &TokensIteratorState) -> Option<usize> {
    let mut to = state.index;

    loop {
        if to >= state.tokens.len() {
            return None;
        }

        if state.seen.contains(&to) {
            to += 1;
            continue;
        }

        if to >= state.tokens.len() {
            return None;
        }

        return Some(to);
    }
}

fn next<'me, 'content>(
    iterator: &'me mut TokensIterator<'content>,
) -> Option<&'content SpannedToken> {
    let next = next_index(&iterator.state);
    let len = iterator.len();

    match next {
        None => {
            iterator.move_to(len);
            None
        }

        Some(index) => {
            iterator.move_to(index + 1);
            Some(&iterator.state.tokens[index])
        }
    }
}
