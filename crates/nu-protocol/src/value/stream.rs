use crate::*;
use std::{
    fmt::Debug,
    sync::{atomic::AtomicBool, Arc},
};

pub struct RawStream {
    pub stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
    pub leftover: Vec<u8>,
    pub ctrlc: Option<Arc<AtomicBool>>,
    pub is_binary: bool,
    pub span: Span,
    pub known_size: Option<u64>, // (bytes)
}

impl RawStream {
    pub fn new(
        stream: Box<dyn Iterator<Item = Result<Vec<u8>, ShellError>> + Send + 'static>,
        ctrlc: Option<Arc<AtomicBool>>,
        span: Span,
        known_size: Option<u64>,
    ) -> Self {
        Self {
            stream,
            leftover: vec![],
            ctrlc,
            is_binary: false,
            span,
            known_size,
        }
    }

    pub fn into_bytes(self) -> Result<Spanned<Vec<u8>>, ShellError> {
        let mut output = vec![];

        for item in self.stream {
            if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
                break;
            }
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
        let ctrlc = &self.ctrlc.clone();

        for item in self {
            if nu_utils::ctrl_c::was_pressed(ctrlc) {
                break;
            }
            output.push_str(&item?.coerce_into_string()?);
        }

        Ok(Spanned { item: output, span })
    }

    pub fn chain(self, stream: RawStream) -> RawStream {
        RawStream {
            stream: Box::new(self.stream.chain(stream.stream)),
            leftover: self.leftover.into_iter().chain(stream.leftover).collect(),
            ctrlc: self.ctrlc,
            is_binary: self.is_binary,
            span: self.span,
            known_size: self.known_size,
        }
    }

    pub fn drain(self) -> Result<(), ShellError> {
        for next in self {
            match next {
                Ok(val) => {
                    if let Value::Error { error, .. } = val {
                        return Err(*error);
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
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
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            return None;
        }

        // If we know we're already binary, just output that
        if self.is_binary {
            self.stream.next().map(|buffer| {
                buffer.map(|mut v| {
                    if !self.leftover.is_empty() {
                        for b in self.leftover.drain(..).rev() {
                            v.insert(0, b);
                        }
                    }
                    Value::binary(v, self.span)
                })
            })
        } else {
            // We *may* be text. We're only going to try utf-8. Other decodings
            // needs to be taken as binary first, then passed through `decode`.
            if let Some(buffer) = self.stream.next() {
                match buffer {
                    Ok(mut v) => {
                        if !self.leftover.is_empty() {
                            while let Some(b) = self.leftover.pop() {
                                v.insert(0, b);
                            }
                        }

                        match String::from_utf8(v.clone()) {
                            Ok(s) => {
                                // Great, we have a complete string, let's output it
                                Some(Ok(Value::string(s, self.span)))
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
                                    Some(Ok(Value::binary(v, self.span)))
                                } else {
                                    // Okay, we have a tiny bit of error at the end of the buffer. This could very well be
                                    // a character that spans two frames. Since this is the case, remove the error from
                                    // the current frame an dput it in the leftover buffer.
                                    self.leftover = v[err.utf8_error().valid_up_to()..].to_vec();

                                    let buf = v[0..err.utf8_error().valid_up_to()].to_vec();

                                    match String::from_utf8(buf) {
                                        Ok(s) => Some(Ok(Value::string(s, self.span))),
                                        Err(_) => {
                                            // Something is definitely wrong. Switch to binary, and stay there
                                            self.is_binary = true;
                                            Some(Ok(Value::binary(v, self.span)))
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            } else if !self.leftover.is_empty() {
                let output = Ok(Value::binary(self.leftover.clone(), self.span));
                self.leftover.clear();

                Some(output)
            } else {
                None
            }
        }
    }
}

/// A potentially infinite stream of values, optionally with a mean to send a Ctrl-C signal to stop
/// the stream from continuing.
///
/// In practice, a "stream" here means anything which can be iterated and produce Values as it iterates.
/// Like other iterators in Rust, observing values from this stream will drain the items as you view them
/// and the stream cannot be replayed.
pub struct ListStream {
    pub stream: Box<dyn Iterator<Item = Value> + Send + 'static>,
    pub ctrlc: Option<Arc<AtomicBool>>,
    first_guard: bool,
}

impl ListStream {
    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.map(|x: Value| x.to_expanded_string(", ", config))
            .collect::<Vec<String>>()
            .join(separator)
    }

    pub fn drain(self) -> Result<(), ShellError> {
        for next in self {
            if let Value::Error { error, .. } = next {
                return Err(*error);
            }
        }
        Ok(())
    }

    pub fn from_stream(
        input: impl Iterator<Item = Value> + Send + 'static,
        ctrlc: Option<Arc<AtomicBool>>,
    ) -> ListStream {
        ListStream {
            stream: Box::new(input),
            ctrlc,
            first_guard: true,
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
        // We need to check `first_guard` to guarantee that it always have something to return in
        // underlying stream.
        //
        // A realworld example is running an external commands, which have an `exit_code`
        // ListStream.
        // When we press ctrl-c, the external command receives the signal too, if we don't have
        // `first_guard`, the `exit_code` ListStream will return Nothing, which is not expected
        if self.first_guard {
            self.first_guard = false;
            return self.stream.next();
        }
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            self.stream.next()
        }
    }
}
