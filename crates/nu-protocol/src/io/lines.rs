use crate::{IntoSpanned, ShellError, Span, Value};
use std::{
    io::{self, BufRead},
    sync::{atomic::AtomicBool, Arc},
};

struct ByteLines<R: BufRead>(R);

impl<R: BufRead> ByteLines<R> {
    pub fn new(read: R) -> Self {
        Self(read)
    }
}

impl<R: BufRead> Iterator for ByteLines<R> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        // `read_until` will never stop reading unless `\n` or EOF is encountered,
        // so we may want to limit the number of bytes by using `take` as the Rust docs suggest.
        // let capacity = self.0.capacity() as u64;
        // let mut reader = (&mut self.0).take(capacity);
        let reader = &mut self.0;
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) => None,
            Ok(_) => {
                if buf.ends_with(&[b'\n']) {
                    let _ = buf.pop();
                    if buf.ends_with(&[b'\r']) {
                        let _ = buf.pop();
                    }
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

pub struct Lines<R: BufRead> {
    lines: ByteLines<R>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl<R: BufRead> Lines<R> {
    pub fn new(reader: R, span: Span, ctrlc: Option<Arc<AtomicBool>>) -> Self {
        Self {
            lines: ByteLines::new(reader),
            span,
            ctrlc,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl<R: BufRead> Iterator for Lines<R> {
    type Item = Result<Vec<u8>, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.lines.next() {
                Some(Ok(line)) => Some(Ok(line)),
                Some(Err(err)) => Some(Err(err.into_spanned(self.span).into())),
                None => None,
            }
        }
    }
}

pub struct Values<R: BufRead> {
    lines: ByteLines<R>,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
}

impl<R: BufRead> Values<R> {
    pub fn new(reader: R, span: Span, ctrlc: Option<Arc<AtomicBool>>) -> Self {
        Self {
            lines: ByteLines::new(reader),
            span,
            ctrlc,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }
}

impl<R: BufRead> Iterator for Values<R> {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.lines.next() {
                Some(Ok(line)) => Some(Ok(match String::from_utf8(line) {
                    Ok(str) => Value::string(str, self.span),
                    Err(err) => Value::binary(err.into_bytes(), self.span),
                })),
                Some(Err(err)) => Some(Err(err.into_spanned(self.span).into())),
                None => None,
            }
        }
    }
}
