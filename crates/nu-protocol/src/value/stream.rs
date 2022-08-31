use crate::*;
use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct RawStream {
    pub stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
    pub leftover: Vec<u8>,
    pub ctrlc: Option<Arc<AtomicBool>>,
    pub is_binary: bool,
    pub span: Span,
}

impl RawStream {
    pub fn new(
        stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
        ctrlc: Option<Arc<AtomicBool>>,
        span: Span,
    ) -> Self {
        Self {
            stream,
            leftover: vec![],
            ctrlc,
            is_binary: false,
            span,
        }
    }

    pub fn into_bytes(self) -> Result<Spanned<Vec<u8>>, ShellError> {
        let mut output = vec![];

        for item in self.stream {
            output.extend(item?);
        }

        Ok(Spanned {
            item: output,
            span: self.span,
        })
    }

    pub fn into_string(self) -> Result<Spanned<String>, ShellError> {
        let mut output = String::new();
        let span = self.span;

        for item in self {
            output.push_str(&item?.as_string()?);
        }

        Ok(Spanned { item: output, span })
    }
}
impl Debug for RawStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawStream").finish()
    }
}
impl Iterator for RawStream {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        // If we know we're already binary, just output that
        if self.is_binary {
            match self.stream.next() {
                Some(buffer) => match buffer {
                    Ok(mut v) => {
                        if !self.leftover.is_empty() {
                            while let Some(b) = self.leftover.pop() {
                                v.insert(0, b);
                            }
                        }
                        Some(Ok(Value::Binary {
                            val: v,
                            span: self.span,
                        }))
                    }
                    Err(e) => Some(Err(e)),
                },
                None => None,
            }
        } else {
            // We *may* be text. We're only going to try utf-8. Other decodings
            // needs to be taken as binary first, then passed through `decode`.
            match self.stream.next() {
                Some(buffer) => match buffer {
                    Ok(mut v) => {
                        if !self.leftover.is_empty() {
                            while let Some(b) = self.leftover.pop() {
                                v.insert(0, b);
                            }
                        }

                        match String::from_utf8(v.clone()) {
                            Ok(s) => {
                                // Great, we have a complete string, let's output it
                                Some(Ok(Value::String {
                                    val: s,
                                    span: self.span,
                                }))
                            }
                            Err(err) => {
                                // Okay, we *might* have a string but we've also got some errors
                                if v.is_empty() {
                                    // We can just end here
                                    None
                                } else if v.len() > 3
                                    && (v.len() - err.utf8_error().valid_up_to() > 3)
                                {
                                    // As UTF-8 characters are max 4 bytes, if we have more than that in error we know
                                    // that it's not just a character spanning two frames.
                                    // We now know we are definitely binary, so switch to binary and stay there.
                                    self.is_binary = true;
                                    Some(Ok(Value::Binary {
                                        val: v,
                                        span: self.span,
                                    }))
                                } else {
                                    // Okay, we have a tiny bit of error at the end of the buffer. This could very well be
                                    // a character that spans two frames. Since this is the case, remove the error from
                                    // the current frame an dput it in the leftover buffer.
                                    self.leftover = v[err.utf8_error().valid_up_to()..].to_vec();

                                    let buf = v[0..err.utf8_error().valid_up_to()].to_vec();

                                    match String::from_utf8(buf) {
                                        Ok(s) => Some(Ok(Value::String {
                                            val: s,
                                            span: self.span,
                                        })),
                                        Err(_) => {
                                            // Something is definitely wrong. Switch to binary, and stay there
                                            self.is_binary = true;
                                            Some(Ok(Value::Binary {
                                                val: v,
                                                span: self.span,
                                            }))
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => Some(Err(e)),
                },
                None => {
                    if !self.leftover.is_empty() {
                        let output = Ok(Value::Binary {
                            val: self.leftover.clone(),
                            span: self.span,
                        });
                        self.leftover.clear();

                        Some(output)
                    } else {
                        None
                    }
                }
            }
        }
    }
}

/// A potentially infinite stream of values, optinally with a mean to send a Ctrl-C signal to stop
/// the stream from continuing.
///
/// In practice, a "stream" here means anything which can be iterated and produce Values as it iterates.
/// Like other iterators in Rust, observing values from this stream will drain the items as you view them
/// and the stream cannot be replayed.
pub struct ListStream {
    pub stream: Box<dyn Iterator<Item = Value> + Send + 'static>,
    pub ctrlc: Option<Arc<AtomicBool>>,
}

impl ListStream {
    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.map(|x: Value| x.into_string(", ", config))
            .collect::<Vec<String>>()
            .join(separator)
    }

    pub fn from_stream(
        input: impl Iterator<Item = Value> + Send + 'static,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> ListStream {
        ListStream {
            stream: Box::new(input),
            ctrlc,
        }
    }
}

impl Debug for ListStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListStream").finish()
    }
}

impl Iterator for ListStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ctrlc) = &self.ctrlc {
            if ctrlc.load(Ordering::SeqCst) {
                None
            } else {
                self.stream.next()
            }
        } else {
            self.stream.next()
        }
    }
}
