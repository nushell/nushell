pub(crate) mod debug;

use self::debug::{ColorTracer, ExpandTracer};
use crate::errors::ShellError;
#[cfg(coloring_in_tokens)]
use crate::parser::hir::syntax_shape::FlatShape;
use crate::parser::hir::Expression;
use crate::parser::TokenNode;
use crate::prelude::*;
use crate::{Span, Spanned, SpannedItem};
#[allow(unused)]
use getset::{Getters, MutGetters};

cfg_if::cfg_if! {
    if #[cfg(coloring_in_tokens)] {
        #[derive(Getters, Debug)]
        pub struct TokensIteratorState<'content> {
            tokens: &'content [TokenNode],
            span: Span,
            skip_ws: bool,
            index: usize,
            seen: indexmap::IndexSet<usize>,
            #[get = "pub"]
            shapes: Vec<Spanned<FlatShape>>,
        }
    } else {
        #[derive(Getters, Debug)]
        pub struct TokensIteratorState<'content> {
            tokens: &'content [TokenNode],
            span: Span,
            skip_ws: bool,
            index: usize,
            seen: indexmap::IndexSet<usize>,
        }
    }
}

#[derive(Getters, MutGetters, Debug)]
pub struct TokensIterator<'content> {
    #[get = "pub"]
    #[get_mut = "pub"]
    state: TokensIteratorState<'content>,
    #[get = "pub"]
    #[get_mut = "pub"]
    color_tracer: ColorTracer,
    #[get = "pub"]
    #[get_mut = "pub"]
    expand_tracer: ExpandTracer,
}

#[derive(Debug)]
pub struct Checkpoint<'content, 'me> {
    pub(crate) iterator: &'me mut TokensIterator<'content>,
    index: usize,
    seen: indexmap::IndexSet<usize>,
    #[cfg(coloring_in_tokens)]
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
            #[cfg(coloring_in_tokens)]
            state.shapes.truncate(self.shape_start);
        }
    }
}

#[derive(Debug)]
pub struct Peeked<'content, 'me> {
    pub(crate) node: Option<&'content TokenNode>,
    iterator: &'me mut TokensIterator<'content>,
    from: usize,
    to: usize,
}

impl<'content, 'me> Peeked<'content, 'me> {
    pub fn commit(&mut self) -> Option<&'content TokenNode> {
        let Peeked {
            node,
            iterator,
            from,
            to,
        } = self;

        let node = (*node)?;
        iterator.commit(*from, *to);
        Some(node)
    }

    pub fn not_eof(self, expected: &'static str) -> Result<PeekedNode<'content, 'me>, ParseError> {
        match self.node {
            None => Err(ParseError::unexpected_eof(
                expected,
                self.iterator.eof_span(),
            )),
            Some(node) => Ok(PeekedNode {
                node,
                iterator: self.iterator,
                from: self.from,
                to: self.to,
            }),
        }
    }

    pub fn type_error(&self, expected: &'static str) -> ParseError {
        peek_error(&self.node, self.iterator.eof_span(), expected)
    }
}

#[derive(Debug)]
pub struct PeekedNode<'content, 'me> {
    pub(crate) node: &'content TokenNode,
    iterator: &'me mut TokensIterator<'content>,
    from: usize,
    to: usize,
}

impl<'content, 'me> PeekedNode<'content, 'me> {
    pub fn commit(self) -> &'content TokenNode {
        let PeekedNode {
            node,
            iterator,
            from,
            to,
        } = self;

        iterator.commit(from, to);
        node
    }

    pub fn rollback(self) {}

    pub fn type_error(&self, expected: &'static str) -> ParseError {
        peek_error(&Some(self.node), self.iterator.eof_span(), expected)
    }
}

pub fn peek_error(node: &Option<&TokenNode>, eof_span: Span, expected: &'static str) -> ParseError {
    match node {
        None => ParseError::unexpected_eof(expected, eof_span),
        Some(node) => ParseError::mismatch(expected, node.type_name().spanned(node.span())),
    }
}

impl<'content> TokensIterator<'content> {
    pub fn new(
        items: &'content [TokenNode],
        span: Span,
        skip_ws: bool,
    ) -> TokensIterator<'content> {
        TokensIterator {
            state: TokensIteratorState {
                tokens: items,
                span,
                skip_ws,
                index: 0,
                seen: indexmap::IndexSet::new(),
                #[cfg(coloring_in_tokens)]
                shapes: vec![],
            },
            color_tracer: ColorTracer::new(),
            expand_tracer: ExpandTracer::new(),
        }
    }

    pub fn all(tokens: &'content [TokenNode], span: Span) -> TokensIterator<'content> {
        TokensIterator::new(tokens, span, false)
    }

    pub fn len(&self) -> usize {
        self.state.tokens.len()
    }

    pub fn spanned<T>(
        &mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> T,
    ) -> Spanned<T> {
        let start = self.span_at_cursor();

        let result = block(self);

        let end = self.span_at_cursor();

        result.spanned(start.until(end))
    }

    #[cfg(coloring_in_tokens)]
    pub fn color_shape(&mut self, shape: Spanned<FlatShape>) {
        self.with_color_tracer(|_, tracer| tracer.add_shape(shape));
        self.state.shapes.push(shape);
    }

    #[cfg(coloring_in_tokens)]
    pub fn mutate_shapes(&mut self, block: impl FnOnce(&mut Vec<Spanned<FlatShape>>)) {
        let new_shapes: Vec<Spanned<FlatShape>> = {
            let shapes = &mut self.state.shapes;
            let len = shapes.len();
            block(shapes);
            (len..(shapes.len())).map(|i| shapes[i]).collect()
        };

        self.with_color_tracer(|_, tracer| {
            for shape in new_shapes {
                tracer.add_shape(shape)
            }
        });
    }

    #[cfg(coloring_in_tokens)]
    pub fn silently_mutate_shapes(&mut self, block: impl FnOnce(&mut Vec<Spanned<FlatShape>>)) {
        let shapes = &mut self.state.shapes;
        block(shapes);
    }

    #[cfg(coloring_in_tokens)]
    pub fn sort_shapes(&mut self) {
        // This is pretty dubious, but it works. We should look into a better algorithm that doesn't end up requiring
        // this solution.

        self.state
            .shapes
            .sort_by(|a, b| a.span.start().cmp(&b.span.start()));
    }

    #[cfg(coloring_in_tokens)]
    pub fn child<'me, T>(
        &'me mut self,
        tokens: Spanned<&'me [TokenNode]>,
        block: impl FnOnce(&mut TokensIterator<'me>) -> T,
    ) -> T {
        let mut shapes = vec![];
        std::mem::swap(&mut shapes, &mut self.state.shapes);

        let mut color_tracer = ColorTracer::new();
        std::mem::swap(&mut color_tracer, &mut self.color_tracer);

        let mut expand_tracer = ExpandTracer::new();
        std::mem::swap(&mut expand_tracer, &mut self.expand_tracer);

        let mut iterator = TokensIterator {
            state: TokensIteratorState {
                tokens: tokens.item,
                span: tokens.span,
                skip_ws: false,
                index: 0,
                seen: indexmap::IndexSet::new(),
                shapes,
            },
            color_tracer,
            expand_tracer,
        };

        let result = block(&mut iterator);

        std::mem::swap(&mut iterator.state.shapes, &mut self.state.shapes);
        std::mem::swap(&mut iterator.color_tracer, &mut self.color_tracer);
        std::mem::swap(&mut iterator.expand_tracer, &mut self.expand_tracer);

        result
    }

    #[cfg(not(coloring_in_tokens))]
    pub fn child<'me, T>(
        &'me mut self,
        tokens: Spanned<&'me [TokenNode]>,
        block: impl FnOnce(&mut TokensIterator<'me>) -> T,
    ) -> T {
        let mut color_tracer = ColorTracer::new();
        std::mem::swap(&mut color_tracer, &mut self.color_tracer);

        let mut expand_tracer = ExpandTracer::new();
        std::mem::swap(&mut expand_tracer, &mut self.expand_tracer);

        let mut iterator = TokensIterator {
            state: TokensIteratorState {
                tokens: tokens.item,
                span: tokens.span,
                skip_ws: false,
                index: 0,
                seen: indexmap::IndexSet::new(),
            },
            color_tracer,
            expand_tracer,
        };

        let result = block(&mut iterator);

        std::mem::swap(&mut iterator.color_tracer, &mut self.color_tracer);
        std::mem::swap(&mut iterator.expand_tracer, &mut self.expand_tracer);

        result
    }

    pub fn with_color_tracer(
        &mut self,
        block: impl FnOnce(&mut TokensIteratorState, &mut ColorTracer),
    ) {
        let state = &mut self.state;
        let color_tracer = &mut self.color_tracer;

        block(state, color_tracer)
    }

    pub fn with_expand_tracer(
        &mut self,
        block: impl FnOnce(&mut TokensIteratorState, &mut ExpandTracer),
    ) {
        let state = &mut self.state;
        let tracer = &mut self.expand_tracer;

        block(state, tracer)
    }

    #[cfg(coloring_in_tokens)]
    pub fn color_frame<T>(
        &mut self,
        desc: &'static str,
        block: impl FnOnce(&mut TokensIterator) -> T,
    ) -> T {
        self.with_color_tracer(|_, tracer| tracer.start(desc));

        let result = block(self);

        self.with_color_tracer(|_, tracer| {
            tracer.success();
        });

        result
    }

    pub fn expand_frame<T>(
        &mut self,
        desc: &'static str,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, ParseError>,
    ) -> Result<T, ParseError>
    where
        T: std::fmt::Debug + FormatDebug + Clone + HasFallibleSpan + 'static,
    {
        self.with_expand_tracer(|_, tracer| tracer.start(desc));

        let result = block(self);

        self.with_expand_tracer(|_, tracer| match &result {
            Ok(result) => {
                tracer.add_result(Box::new(result.clone()));
                tracer.success();
            }

            Err(err) => tracer.failed(err),
        });

        result
    }

    pub fn expand_expr_frame(
        &mut self,
        desc: &'static str,
        block: impl FnOnce(&mut TokensIterator) -> Result<Expression, ParseError>,
    ) -> Result<Expression, ParseError> {
        self.with_expand_tracer(|_, tracer| tracer.start(desc));

        let result = block(self);

        self.with_expand_tracer(|_, tracer| match &result {
            Ok(expr) => {
                tracer.add_expr(expr.clone());
                tracer.success()
            }

            Err(err) => tracer.failed(err),
        });

        result
    }

    pub fn color_fallible_frame<T>(
        &mut self,
        desc: &'static str,
        block: impl FnOnce(&mut TokensIterator) -> Result<T, ShellError>,
    ) -> Result<T, ShellError> {
        self.with_color_tracer(|_, tracer| tracer.start(desc));

        if self.at_end() {
            self.with_color_tracer(|_, tracer| tracer.eof_frame());
            return Err(ShellError::unexpected_eof("coloring", Tag::unknown()));
        }

        let result = block(self);

        self.with_color_tracer(|_, tracer| match &result {
            Ok(_) => {
                tracer.success();
            }

            Err(err) => tracer.failed(err),
        });

        result
    }

    /// Use a checkpoint when you need to peek more than one token ahead, but can't be sure
    /// that you'll succeed.
    pub fn checkpoint<'me>(&'me mut self) -> Checkpoint<'content, 'me> {
        let state = &mut self.state;

        let index = state.index;
        #[cfg(coloring_in_tokens)]
        let shape_start = state.shapes.len();
        let seen = state.seen.clone();

        Checkpoint {
            iterator: self,
            index,
            seen,
            committed: false,
            #[cfg(coloring_in_tokens)]
            shape_start,
        }
    }

    /// Use a checkpoint when you need to peek more than one token ahead, but can't be sure
    /// that you'll succeed.
    pub fn atomic<'me, T>(
        &'me mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, ShellError>,
    ) -> Result<T, ShellError> {
        let state = &mut self.state;

        let index = state.index;
        #[cfg(coloring_in_tokens)]
        let shape_start = state.shapes.len();
        let seen = state.seen.clone();

        let checkpoint = Checkpoint {
            iterator: self,
            index,
            seen,
            committed: false,
            #[cfg(coloring_in_tokens)]
            shape_start,
        };

        let value = block(checkpoint.iterator)?;

        checkpoint.commit();
        return Ok(value);
    }

    /// Use a checkpoint when you need to peek more than one token ahead, but can't be sure
    /// that you'll succeed.
    pub fn atomic_parse<'me, T>(
        &'me mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let state = &mut self.state;

        let index = state.index;
        #[cfg(coloring_in_tokens)]
        let shape_start = state.shapes.len();
        let seen = state.seen.clone();

        let checkpoint = Checkpoint {
            iterator: self,
            index,
            seen,
            committed: false,
            #[cfg(coloring_in_tokens)]
            shape_start,
        };

        let value = block(checkpoint.iterator)?;

        checkpoint.commit();
        return Ok(value);
    }

    #[cfg(coloring_in_tokens)]
    /// Use a checkpoint when you need to peek more than one token ahead, but can't be sure
    /// that you'll succeed.
    pub fn atomic_returning_shapes<'me, T>(
        &'me mut self,
        block: impl FnOnce(&mut TokensIterator<'content>) -> Result<T, ShellError>,
    ) -> (Result<T, ShellError>, Vec<Spanned<FlatShape>>) {
        let index = self.state.index;
        let mut shapes = vec![];

        let seen = self.state.seen.clone();
        std::mem::swap(&mut self.state.shapes, &mut shapes);

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
                return (Err(err), vec![]);
            }

            Ok(value) => value,
        };

        checkpoint.commit();
        std::mem::swap(&mut self.state.shapes, &mut shapes);
        return (Ok(value), shapes);
    }

    fn eof_span(&self) -> Span {
        Span::new(self.state.span.end(), self.state.span.end())
    }

    pub fn typed_span_at_cursor(&mut self) -> Spanned<&'static str> {
        let next = self.peek_any();

        match next.node {
            None => "end".spanned(self.eof_span()),
            Some(node) => node.spanned_type_name(),
        }
    }

    pub fn span_at_cursor(&mut self) -> Span {
        let next = self.peek_any();

        match next.node {
            None => self.eof_span(),
            Some(node) => node.span(),
        }
    }

    pub fn remove(&mut self, position: usize) {
        self.state.seen.insert(position);
    }

    pub fn at_end(&self) -> bool {
        peek(self, self.state.skip_ws).is_none()
    }

    pub fn at_end_possible_ws(&self) -> bool {
        peek(self, true).is_none()
    }

    pub fn advance(&mut self) {
        self.state.seen.insert(self.state.index);
        self.state.index += 1;
    }

    pub fn extract<T>(&mut self, f: impl Fn(&TokenNode) -> Option<T>) -> Option<(usize, T)> {
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

        None
    }

    pub fn move_to(&mut self, pos: usize) {
        self.state.index = pos;
    }

    pub fn restart(&mut self) {
        self.state.index = 0;
    }

    // pub fn clone(&self) -> TokensIterator<'content> {
    //     let state = &self.state;
    //     TokensIterator {
    //         state: TokensIteratorState {
    //             tokens: state.tokens,
    //             span: state.span,
    //             index: state.index,
    //             seen: state.seen.clone(),
    //             skip_ws: state.skip_ws,
    //             #[cfg(coloring_in_tokens)]
    //             shapes: state.shapes.clone(),
    //         },
    //         color_tracer: self.color_tracer.clone(),
    //         expand_tracer: self.expand_tracer.clone(),
    //     }
    // }

    // Peek the next token, not including whitespace
    pub fn peek_non_ws<'me>(&'me mut self) -> Peeked<'content, 'me> {
        start_next(self, true)
    }

    // Peek the next token, including whitespace
    pub fn peek_any<'me>(&'me mut self) -> Peeked<'content, 'me> {
        start_next(self, false)
    }

    // Peek the next token, including whitespace, but not EOF
    pub fn peek_any_token<'me, T>(
        &'me mut self,
        expected: &'static str,
        block: impl FnOnce(&'content TokenNode) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let peeked = start_next(self, false);
        let peeked = peeked.not_eof(expected);

        match peeked {
            Err(err) => return Err(err),
            Ok(peeked) => match block(peeked.node) {
                Err(err) => return Err(err),
                Ok(val) => {
                    peeked.commit();
                    return Ok(val);
                }
            },
        }
    }

    fn commit(&mut self, from: usize, to: usize) {
        for index in from..to {
            self.state.seen.insert(index);
        }

        self.state.index = to;
    }

    pub fn pos(&self, skip_ws: bool) -> Option<usize> {
        peek_pos(self, skip_ws)
    }

    pub fn debug_remaining(&self) -> Vec<TokenNode> {
        // TODO: TODO: TODO: Clean up
        vec![]
        // let mut tokens = self.clone();
        // tokens.restart();
        // tokens.cloned().collect()
    }
}

impl<'content> Iterator for TokensIterator<'content> {
    type Item = &'content TokenNode;

    fn next(&mut self) -> Option<Self::Item> {
        next(self, self.state.skip_ws)
    }
}

fn peek<'content, 'me>(
    iterator: &'me TokensIterator<'content>,
    skip_ws: bool,
) -> Option<&'me TokenNode> {
    let state = iterator.state();

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

        let node = &state.tokens[to];

        match node {
            TokenNode::Whitespace(_) if skip_ws => {
                to += 1;
            }
            _ => {
                return Some(node);
            }
        }
    }
}

fn peek_pos<'content, 'me>(
    iterator: &'me TokensIterator<'content>,
    skip_ws: bool,
) -> Option<usize> {
    let state = iterator.state();

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

        let node = &state.tokens[to];

        match node {
            TokenNode::Whitespace(_) if skip_ws => {
                to += 1;
            }
            _ => return Some(to),
        }
    }
}

fn start_next<'content, 'me>(
    iterator: &'me mut TokensIterator<'content>,
    skip_ws: bool,
) -> Peeked<'content, 'me> {
    let state = iterator.state();

    let from = state.index;
    let mut to = state.index;

    loop {
        if to >= state.tokens.len() {
            return Peeked {
                node: None,
                iterator,
                from,
                to,
            };
        }

        if state.seen.contains(&to) {
            to += 1;
            continue;
        }

        if to >= state.tokens.len() {
            return Peeked {
                node: None,
                iterator,
                from,
                to,
            };
        }

        let node = &state.tokens[to];

        match node {
            TokenNode::Whitespace(_) if skip_ws => {
                to += 1;
            }
            _ => {
                to += 1;
                return Peeked {
                    node: Some(node),
                    iterator,
                    from,
                    to,
                };
            }
        }
    }
}

fn next<'me, 'content>(
    iterator: &'me mut TokensIterator<'content>,
    skip_ws: bool,
) -> Option<&'content TokenNode> {
    loop {
        if iterator.state().index >= iterator.state().tokens.len() {
            return None;
        }

        if iterator.state().seen.contains(&iterator.state().index) {
            iterator.advance();
            continue;
        }

        if iterator.state().index >= iterator.state().tokens.len() {
            return None;
        }

        match &iterator.state().tokens[iterator.state().index] {
            TokenNode::Whitespace(_) if skip_ws => {
                iterator.advance();
            }
            other => {
                iterator.advance();
                return Some(other);
            }
        }
    }
}
