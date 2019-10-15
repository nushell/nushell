pub(crate) mod debug;

use crate::errors::ShellError;
#[cfg(coloring_in_tokens)]
use crate::parser::hir::syntax_shape::FlatShape;
use crate::parser::TokenNode;
use crate::{Span, Spanned, SpannedItem};
#[allow(unused)]
use getset::Getters;

#[derive(Getters, Debug)]
pub struct TokensIterator<'content> {
    tokens: &'content [TokenNode],
    span: Span,
    skip_ws: bool,
    index: usize,
    seen: indexmap::IndexSet<usize>,
    #[cfg(coloring_in_tokens)]
    #[get = "pub"]
    shapes: Vec<Spanned<FlatShape>>,
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
            self.iterator.index = self.index;
            self.iterator.seen = self.seen.clone();
            #[cfg(coloring_in_tokens)]
            self.iterator.shapes.truncate(self.shape_start);
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

    pub fn not_eof(
        self,
        expected: impl Into<String>,
    ) -> Result<PeekedNode<'content, 'me>, ShellError> {
        match self.node {
            None => Err(ShellError::unexpected_eof(
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

    pub fn type_error(&self, expected: impl Into<String>) -> ShellError {
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

    pub fn type_error(&self, expected: impl Into<String>) -> ShellError {
        peek_error(&Some(self.node), self.iterator.eof_span(), expected)
    }
}

pub fn peek_error(
    node: &Option<&TokenNode>,
    eof_span: Span,
    expected: impl Into<String>,
) -> ShellError {
    match node {
        None => ShellError::unexpected_eof(expected, eof_span),
        Some(node) => ShellError::type_error(expected, node.tagged_type_name()),
    }
}

impl<'content> TokensIterator<'content> {
    pub fn new(
        items: &'content [TokenNode],
        span: Span,
        skip_ws: bool,
    ) -> TokensIterator<'content> {
        TokensIterator {
            tokens: items,
            span,
            skip_ws,
            index: 0,
            seen: indexmap::IndexSet::new(),
            #[cfg(coloring_in_tokens)]
            shapes: vec![],
        }
    }

    pub fn all(tokens: &'content [TokenNode], span: Span) -> TokensIterator<'content> {
        TokensIterator::new(tokens, span, false)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
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
        self.shapes.push(shape);
    }

    #[cfg(coloring_in_tokens)]
    pub fn mut_shapes(&mut self) -> &mut Vec<Spanned<FlatShape>> {
        &mut self.shapes
    }

    #[cfg(coloring_in_tokens)]
    pub fn child<T>(
        &mut self,
        tokens: Spanned<&'content [TokenNode]>,
        block: impl FnOnce(&mut TokensIterator) -> T,
    ) -> T {
        let mut shapes = vec![];
        std::mem::swap(&mut shapes, &mut self.shapes);

        let mut iterator = TokensIterator {
            tokens: tokens.item,
            span: tokens.span,
            skip_ws: false,
            index: 0,
            seen: indexmap::IndexSet::new(),
            shapes,
        };

        let result = block(&mut iterator);

        std::mem::swap(&mut iterator.shapes, &mut self.shapes);

        result
    }

    /// Use a checkpoint when you need to peek more than one token ahead, but can't be sure
    /// that you'll succeed.
    pub fn checkpoint<'me>(&'me mut self) -> Checkpoint<'content, 'me> {
        let index = self.index;
        #[cfg(coloring_in_tokens)]
        let shape_start = self.shapes.len();
        let seen = self.seen.clone();

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
        let index = self.index;
        #[cfg(coloring_in_tokens)]
        let shape_start = self.shapes.len();
        let seen = self.seen.clone();

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
        let index = self.index;
        let mut shapes = vec![];

        let seen = self.seen.clone();
        std::mem::swap(&mut self.shapes, &mut shapes);

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
                std::mem::swap(&mut self.shapes, &mut shapes);
                return (Err(err), vec![]);
            }

            Ok(value) => value,
        };

        checkpoint.commit();
        std::mem::swap(&mut self.shapes, &mut shapes);
        return (Ok(value), shapes);
    }

    fn eof_span(&self) -> Span {
        Span::new(self.span.end(), self.span.end())
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
        self.seen.insert(position);
    }

    pub fn at_end(&self) -> bool {
        peek(self, self.skip_ws).is_none()
    }

    pub fn at_end_possible_ws(&self) -> bool {
        peek(self, true).is_none()
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

    pub fn clone(&self) -> TokensIterator<'content> {
        TokensIterator {
            tokens: self.tokens,
            span: self.span,
            index: self.index,
            seen: self.seen.clone(),
            skip_ws: self.skip_ws,
            #[cfg(coloring_in_tokens)]
            shapes: self.shapes.clone(),
        }
    }

    // Get the next token, not including whitespace
    pub fn next_non_ws(&mut self) -> Option<&TokenNode> {
        let mut peeked = start_next(self, true);
        peeked.commit()
    }

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
        block: impl FnOnce(&'content TokenNode) -> Result<T, ShellError>,
    ) -> Result<T, ShellError> {
        let peeked = start_next(self, false);
        let peeked = peeked.not_eof("invariant");

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
            self.seen.insert(index);
        }

        self.index = to;
    }

    pub fn pos(&self, skip_ws: bool) -> Option<usize> {
        peek_pos(self, skip_ws)
    }

    pub fn debug_remaining(&self) -> Vec<TokenNode> {
        let mut tokens = self.clone();
        tokens.restart();
        tokens.cloned().collect()
    }
}

impl<'content> Iterator for TokensIterator<'content> {
    type Item = &'content TokenNode;

    fn next(&mut self) -> Option<&'content TokenNode> {
        next(self, self.skip_ws)
    }
}

fn peek<'content, 'me>(
    iterator: &'me TokensIterator<'content>,
    skip_ws: bool,
) -> Option<&'me TokenNode> {
    let mut to = iterator.index;

    loop {
        if to >= iterator.tokens.len() {
            return None;
        }

        if iterator.seen.contains(&to) {
            to += 1;
            continue;
        }

        if to >= iterator.tokens.len() {
            return None;
        }

        let node = &iterator.tokens[to];

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
    let mut to = iterator.index;

    loop {
        if to >= iterator.tokens.len() {
            return None;
        }

        if iterator.seen.contains(&to) {
            to += 1;
            continue;
        }

        if to >= iterator.tokens.len() {
            return None;
        }

        let node = &iterator.tokens[to];

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
    let from = iterator.index;
    let mut to = iterator.index;

    loop {
        if to >= iterator.tokens.len() {
            return Peeked {
                node: None,
                iterator,
                from,
                to,
            };
        }

        if iterator.seen.contains(&to) {
            to += 1;
            continue;
        }

        if to >= iterator.tokens.len() {
            return Peeked {
                node: None,
                iterator,
                from,
                to,
            };
        }

        let node = &iterator.tokens[to];

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
        if iterator.index >= iterator.tokens.len() {
            return None;
        }

        if iterator.seen.contains(&iterator.index) {
            iterator.advance();
            continue;
        }

        if iterator.index >= iterator.tokens.len() {
            return None;
        }

        match &iterator.tokens[iterator.index] {
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
