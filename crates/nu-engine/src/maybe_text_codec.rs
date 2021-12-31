use std::io::{BufRead, BufReader, Error, Read};

use encoding_rs::{Encoding, UTF_8};

#[derive(Debug, Eq, PartialEq)]
pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}

pub struct BufCodecReader<R: Read> {
    encoding: &'static Encoding,
    input: BufReader<R>,
}

impl<R: Read> BufCodecReader<R> {
    /// Wrap the given read implementation with the given encoding. If `None` it falls back to UTF-8.
    pub fn new(input: BufReader<R>, encoding: Option<&'static Encoding>) -> Self {
        BufCodecReader {
            encoding: encoding.unwrap_or(UTF_8),
            input,
        }
    }

    /// Read some input and attempt to decode it using the current encoding.
    /// Returns a `String` if the line can be successfully decoded, or a
    /// `Binary` otherwise.
    pub fn read_some(&mut self) -> Result<Option<StringOrBinary>, Error> {
        let buf = self.input.fill_buf()?;

        if buf.is_empty() {
            return Ok(None);
        }

        let (string, _, replacements) = self.encoding.decode(buf);

        let value = if replacements {
            StringOrBinary::Binary(buf.to_vec())
        } else {
            StringOrBinary::String(string.into_owned())
        };

        let len = buf.len();
        self.input.consume(len);
        Ok(Some(value))
    }

    /// Read the whole buffer and attempt to decode it using the current
    /// encoding. Returns a `String` if the line can be successfully decoded, or
    /// a `Binary` otherwise.
    pub fn read_to_end(mut self) -> Result<StringOrBinary, Error> {
        let mut buf = Vec::new();
        self.input.read_to_end(&mut buf)?;

        let (string, _, replacements) = self.encoding.decode(&buf);

        let value = if replacements {
            StringOrBinary::Binary(buf)
        } else {
            StringOrBinary::String(string.into_owned())
        };

        Ok(value)
    }
}
