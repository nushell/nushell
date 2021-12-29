use std::io::{Error, Read};

use encoding_rs::{Decoder, DecoderResult, Encoding, UTF_8};

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
    decoder: Decoder,
    input: R,
}

impl<R: Read> BufCodecReader<R> {
    /// Wrap the given read implementation with the given encoding. If `None` it falls back to UTF-8.
    pub fn new(input: R, encoding: Option<&'static Encoding>) -> Self {
        BufCodecReader {
            decoder: encoding.unwrap_or(UTF_8).new_decoder(),
            input,
        }
    }

    /// Read the whole buffer into a `String` if it can be successfully decoded,
    /// or a `Binary` if the underlying data cannot be decoded.
    pub fn read_full(mut self) -> Result<StringOrBinary, Error> {
        let mut init = [0u8; OUTPUT_BUFFER_SIZE];

        let mut fallback = Vec::new();
        let mut string = String::new();
        let mut cur = &[][..];

        loop {
            let (result, read) =
                self.decoder
                    .decode_to_string_without_replacement(cur, &mut string, false);
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
                self.decoder
                    .decode_to_string_without_replacement(cur, &mut string, true);
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
