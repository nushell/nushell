use crate::{ByteStreamType, ErrSpan, ShellError, Span, Value};
use std::{
    io::{BufRead, BufReader, Read},
    sync::{atomic::AtomicBool, Arc},
};

#[cfg(test)]
mod tests;

/// Turn a readable stream into [`Value`]s.
///
/// The `Value` type depends on the type of the stream ([`ByteStreamType`]). If `Unknown`, the
/// stream will return strings as long as UTF-8 parsing succeeds, but will start returning binary
/// if it fails.
pub struct Values<R: Read> {
    reader: BufReader<R>,
    pos: u64,
    error: bool,
    span: Span,
    ctrlc: Option<Arc<AtomicBool>>,
    r#type: ByteStreamType,
}

impl<R: Read> Values<R> {
    pub fn new(
        reader: R,
        span: Span,
        ctrlc: Option<Arc<AtomicBool>>,
        r#type: ByteStreamType,
    ) -> Self {
        Self {
            reader: BufReader::new(reader),
            pos: 0,
            error: false,
            span,
            ctrlc,
            r#type,
        }
    }

    pub fn span(&self) -> Span {
        self.span
    }

    fn next_string(&mut self) -> Result<Option<String>, (Vec<u8>, ShellError)> {
        // Get some data from the reader
        let buf = self
            .reader
            .fill_buf()
            .err_span(self.span)
            .map_err(|err| (vec![], ShellError::from(err)))?;

        // If empty, this is EOF
        if buf.is_empty() {
            return Ok(None);
        }

        let mut buf = buf.to_vec();
        let mut consumed = 0;

        // If the buf length is under 4 bytes, it could be invalid, so try to get more
        if buf.len() < 4 {
            consumed += buf.len();
            self.reader.consume(buf.len());
            match self.reader.fill_buf().err_span(self.span) {
                Ok(more_bytes) => buf.extend_from_slice(more_bytes),
                Err(err) => return Err((buf, err.into())),
            }
        }

        // Try to parse utf-8 and decide what to do
        match String::from_utf8(buf) {
            Ok(string) => {
                self.reader.consume(string.len() - consumed);
                self.pos += string.len() as u64;
                Ok(Some(string))
            }
            Err(err) if err.utf8_error().error_len().is_none() => {
                // There is some valid data at the beginning, and this is just incomplete, so just
                // consume that and return it
                let valid_up_to = err.utf8_error().valid_up_to();
                if valid_up_to > consumed {
                    self.reader.consume(valid_up_to - consumed);
                }
                let mut buf = err.into_bytes();
                buf.truncate(valid_up_to);
                buf.shrink_to_fit();
                let string = String::from_utf8(buf)
                    .expect("failed to parse utf-8 even after correcting error");
                self.pos += string.len() as u64;
                Ok(Some(string))
            }
            Err(err) => {
                // There is an error at the beginning and we have no hope of parsing further.
                let shell_error = ShellError::NonUtf8Custom {
                    msg: format!("invalid utf-8 sequence starting at index {}", self.pos),
                    span: self.span,
                };
                let buf = err.into_bytes();
                // We are consuming the entire buf though, because we're returning it in case it
                // will be cast to binary
                if buf.len() > consumed {
                    self.reader.consume(buf.len() - consumed);
                }
                self.pos += buf.len() as u64;
                Err((buf, shell_error))
            }
        }
    }
}

impl<R: Read> Iterator for Values<R> {
    type Item = Result<Value, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error || nu_utils::ctrl_c::was_pressed(&self.ctrlc) {
            None
        } else {
            match self.r#type {
                // Binary should always be binary
                ByteStreamType::Binary => {
                    let buf = match self.reader.fill_buf().err_span(self.span) {
                        Ok(buf) => buf,
                        Err(err) => {
                            self.error = true;
                            return Some(Err(err.into()));
                        }
                    };
                    if !buf.is_empty() {
                        let len = buf.len();
                        let value = Value::binary(buf, self.span);
                        self.reader.consume(len);
                        self.pos += len as u64;
                        Some(Ok(value))
                    } else {
                        None
                    }
                }
                // String produces an error if UTF-8 can't be parsed
                ByteStreamType::String => match self.next_string().transpose()? {
                    Ok(string) => Some(Ok(Value::string(string, self.span))),
                    Err((_, err)) => {
                        self.error = true;
                        Some(Err(err))
                    }
                },
                // For Unknown, we try to create strings, but we switch to binary mode if we
                // fail
                ByteStreamType::Unknown => {
                    match self.next_string().transpose()? {
                        Ok(string) => Some(Ok(Value::string(string, self.span))),
                        Err((buf, _)) if !buf.is_empty() => {
                            // Switch to binary mode
                            self.r#type = ByteStreamType::Binary;
                            Some(Ok(Value::binary(buf, self.span)))
                        }
                        Err((_, err)) => {
                            self.error = true;
                            Some(Err(err))
                        }
                    }
                }
            }
        }
    }
}
