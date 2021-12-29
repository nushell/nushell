use std::io::{BufRead, BufReader, Error, Read};

use encoding_rs::{DecoderResult, Encoding, UTF_8};

#[cfg(not(test))]
const OUTPUT_BUFFER_SIZE: usize = 8192;
#[cfg(test)]
const OUTPUT_BUFFER_SIZE: usize = 4;

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

    /// Read a line of  input and attempt to decode it using the current
    /// encoding. Returns a `String` if the line can be successfully decoded, or
    /// a `Binary` otherwise.
    pub fn read_line(&mut self) -> Result<Option<StringOrBinary>, Error> {
        let mut buf = Vec::new();

        // Using same delimiter as `BufReader::read_line`, but with our own
        // encoding.
        self.input.read_until(b'\n', &mut buf)?;

        if buf.is_empty() {
            return Ok(None);
        }

        let (string, _, replacements) = self.encoding.decode(&buf);

        let value = if replacements {
            StringOrBinary::Binary(buf)
        } else {
            StringOrBinary::String(string.into_owned())
        };

        Ok(Some(value))
    }

    /// Read the whole buffer and attempt to decode it using the current
    /// encoding. Returns a `String` if the line can be successfully decoded, or
    /// a `Binary` otherwise.
    pub fn read_full(mut self) -> Result<StringOrBinary, Error> {
        let mut decoder = self.encoding.new_decoder();

        let mut init = [0u8; OUTPUT_BUFFER_SIZE];

        let mut fallback = Vec::new();
        let mut string = String::new();
        let mut cur = &[][..];

        loop {
            let (result, read) =
                decoder.decode_to_string_without_replacement(cur, &mut string, false);
            cur = &cur[read..];

            match result {
                DecoderResult::InputEmpty => {
                    debug_assert!(cur.is_empty());

                    // Satisfy borrow checker.
                    cur = &[][..];

                    match self.input.read(&mut init[..]) {
                        Ok(0) => {
                            break;
                        }
                        Ok(n) => {
                            fallback.extend(&init[..]);
                            cur = &init[..n];
                        }
                        Err(e) => return Err(e),
                    }
                }
                DecoderResult::OutputFull => {
                    string.reserve(OUTPUT_BUFFER_SIZE);
                }
                DecoderResult::Malformed(..) => {
                    // This is why we maintain `fallback` of all bytes read so
                    // far. We cannot use `string` because this doesn't
                    // necessarily represent the underlying bytes read.
                    if let Err(e) = self.input.read_to_end(&mut fallback) {
                        return Err(e);
                    }

                    return Ok(StringOrBinary::Binary(fallback));
                }
            }
        }

        // Perform last decode call, which again needs to be done in a loop.
        loop {
            let (result, read) =
                decoder.decode_to_string_without_replacement(cur, &mut string, true);
            cur = &cur[read..];

            match result {
                // NB: InputEmpty when last is set to `true` means that decoding
                // successfully completed.
                DecoderResult::InputEmpty => {
                    debug_assert!(cur.is_empty());
                    return Ok(StringOrBinary::String(string));
                }
                DecoderResult::OutputFull => {
                    string.reserve(OUTPUT_BUFFER_SIZE);
                }
                DecoderResult::Malformed(..) => {
                    // This is why we maintain `fallback` of all bytes read so
                    // far. We cannot use `string` because this doesn't
                    // necessarily represent the underlying bytes read.
                    if let Err(e) = self.input.read_to_end(&mut fallback) {
                        return Err(e);
                    }

                    return Ok(StringOrBinary::Binary(fallback));
                }
            }
        }
    }
}
