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
            output.push_str(&item?.as_string()?);
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

    pub fn into_reader<B: Into<u8>>(
        mut self,
        separator: Option<B>,
    ) -> std::io::Result<StreamReader> {
        let separator = separator.map(Into::into);
        let cursor = val_to_cursor(self.next(), separator)?;

        Ok(StreamReader {
            stream: Box::new(self),
            cursor,
            separator,
        })
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
}

impl ListStream {
    pub fn into_string(self, separator: &str, config: &Config) -> String {
        self.map(|x: Value| x.into_string(", ", config))
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
        }
    }

    pub fn into_reader<B: Into<u8>>(
        mut self,
        separator: Option<B>,
    ) -> std::io::Result<StreamReader> {
        let separator = separator.map(Into::into);
        let cursor = val_to_cursor(self.next().map(Ok), separator)?;

        Ok(StreamReader {
            stream: Box::new(self.map(Ok)),
            cursor,
            separator,
        })
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
        if nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            self.stream.next()
        }
    }
}

pub struct StreamReader {
    stream: Box<dyn Iterator<Item = Result<Value, ShellError>> + Send + 'static>,
    cursor: Option<std::io::Cursor<Vec<u8>>>,
    separator: Option<u8>,
}

impl std::io::Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        while let Some(ref mut cursor) = self.cursor {
            let read = cursor.read(buf)?;

            if read > 0 {
                return Ok(read);
            }

            self.cursor = val_to_cursor(self.stream.next(), self.separator)?;
        }

        Ok(0)
    }
}

fn val_to_cursor(
    val: Option<Result<Value, ShellError>>,
    separator: Option<u8>,
) -> std::io::Result<Option<std::io::Cursor<Vec<u8>>>> {
    Ok(match val {
        None => None,
        Some(val) => match val {
            Ok(Value::String { val, .. }) => {
                let mut bytes = val.into_bytes();

                if let Some(sep) = separator {
                    bytes.push(sep);
                }

                Some(std::io::Cursor::new(bytes))
            }
            Ok(Value::Binary { mut val, .. }) => {
                if let Some(sep) = separator {
                    val.push(sep);
                }

                Some(std::io::Cursor::new(val))
            }
            Err(err) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err.to_string(),
                ))
            }
            Ok(val) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("input should be string or binary, got: {}", val.get_type()),
                ));
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_reader_empty() {
        let data: [Value; 0] = [];
        let (mut test_data, _, _) = data
            .into_pipeline_data(None)
            .into_reader::<u8>(Span::unknown(), None)
            .unwrap();

        let out = std::io::read_to_string(&mut test_data).unwrap();

        assert_eq!("".to_owned(), out);
    }

    #[test]
    fn stream_reader_basic() {
        let (mut test_data, _, _) = [
            Value::string("str1".to_owned(), Span::unknown()),
            Value::string("str2".to_owned(), Span::unknown()),
            Value::binary(b"bin1".to_owned(), Span::unknown()),
            Value::binary(b"bin2".to_owned(), Span::unknown()),
        ]
        .into_pipeline_data(None)
        .into_reader(Span::unknown(), Some(b'\n'))
        .unwrap();

        let out = std::io::read_to_string(&mut test_data).unwrap();

        assert_eq!("str1\nstr2\nbin1\nbin2\n".to_owned(), out);
    }

    #[test]
    fn stream_reader_binary() {
        let bytes = vec![0, 1, 2, 3];

        let (mut test_data, _, _) = [
            Value::string("foo".to_owned(), Span::unknown()),
            Value::binary(bytes.clone(), Span::unknown()),
        ]
        .into_pipeline_data(None)
        .into_reader(Span::unknown(), Some(0))
        .unwrap();

        let mut expected = Vec::with_capacity(9);
        expected.extend_from_slice(b"foo\0");
        expected.extend_from_slice(&bytes);
        expected.extend_from_slice(b"\0");

        let mut out_buf = Vec::with_capacity(9);
        let out = std::io::copy(&mut test_data, &mut out_buf).unwrap();

        assert_eq!(9, out);
        assert_eq!(expected, out_buf);
    }

    /// Verify that if a pipeline's value that it's reading into a buffer does
    /// not have space store the whole content that the reader will copy all the
    /// data across successive runs and track state correctly.
    #[test]
    fn stream_reader_binary_small_buffer() {
        let bytes = vec![0, 1, 2, 3];

        let (mut test_data, _, _) = [
            Value::string("fooz".to_owned(), Span::unknown()),
            Value::binary(bytes.clone(), Span::unknown()),
        ]
        .into_pipeline_data(None)
        .into_reader::<u8>(Span::unknown(), None)
        .unwrap();

        let mut expected = Vec::from_iter(b"foo".iter().copied());

        let mut out_buf = [0; 3];
        let mut written = test_data.read(&mut out_buf).unwrap();

        assert_eq!(3, written);
        assert_eq!(expected, out_buf);

        expected = vec![b'z'];
        written = test_data.read(&mut out_buf).unwrap();

        assert_eq!(1, written);
        assert_eq!(expected, &out_buf[..1]);

        expected = vec![0, 1, 2];
        written = test_data.read(&mut out_buf).unwrap();

        assert_eq!(3, written);
        assert_eq!(expected, out_buf);

        expected = vec![3];
        written = test_data.read(&mut out_buf).unwrap();

        assert_eq!(1, written);
        assert_eq!(expected, &out_buf[..1]);

        // last read is empty -- nothing left
        written = test_data.read(&mut out_buf).unwrap();
        assert_eq!(0, written);
    }

    #[test]
    fn stream_reader_invalid_type() {
        let (mut test_data, _, _) = [
            Value::string("str1".to_owned(), Span::unknown()),
            Value::binary(b"bin1".to_owned(), Span::unknown()),
            Value::record(
                record!(
                    "foo".to_owned() => Value::string("bar".to_owned(), Span::unknown()),
                    "baz".to_owned() => Value::int(1, Span::unknown()),
                ),
                Span::unknown(),
            ),
        ]
        .into_pipeline_data(None)
        .into_reader(Span::unknown(), Some(b'\n'))
        .unwrap();

        let out = std::io::read_to_string(&mut test_data).unwrap_err();

        assert_eq!(std::io::ErrorKind::InvalidInput, out.kind());
    }
}
