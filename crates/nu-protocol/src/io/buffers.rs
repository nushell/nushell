use crate::{IntoSpanned, ShellError, Span};
use std::{
    io::{self, BufRead, BufReader, Read},
    sync::{atomic::AtomicBool, Arc},
};

/// Iterates on buffers immediately as they're received, up to a maximum size.
///
/// See [`Buffers`] for an interruptible version with [`ShellError`] errors.
pub struct ByteBuffers<R: Read>(BufReader<R>);

impl<R: Read> ByteBuffers<R> {
    pub fn new(read: R) -> Self {
        Self(BufReader::new(read))
    }
}

impl<R: Read> Iterator for ByteBuffers<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .0
            .fill_buf()
            .map(|buf| {
                if !buf.is_empty() {
                    Some(buf.to_vec())
                } else {
                    None
                }
            })
            .transpose();

        if let Some(Ok(buf)) = &result {
            self.0.consume(buf.len());
        }
        result
    }
}

/// Iterates on buffers immediately as they're received, up to a maximum size.
pub struct Buffers<R: Read> {
    buffers: ByteBuffers<R>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl<R: Read> Buffers<R> {
    pub fn new(reader: R, span: Span, ctrlc: Option<Arc<AtomicBool>>) -> Self {
        Self {
            buffers: ByteBuffers::new(reader),
            span,
            ctrlc,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl<R: Read> Iterator for Buffers<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.buffers.next() {
                Some(Ok(buf)) => Some(Ok(buf)),
                Some(Err(err)) => Some(Err(err.into_spanned(self.span).into())),
                None => None,
            }
        }
    }
}
