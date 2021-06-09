use std::io::{BufRead, BufReader, Read};

use nu_errors::ShellError;

use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};

#[cfg(not(test))]
const OUTPUT_BUFFER_SIZE: usize = 8192;
#[cfg(test)]
const OUTPUT_BUFFER_SIZE: usize = 4;

#[derive(Debug, Eq, PartialEq)]
pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}

pub struct MaybeTextCodec {
    decoder: Decoder,
}

pub struct BufCodecReader<R: Read> {
    maybe_text_codec: MaybeTextCodec,
    input: BufReader<R>,
}

impl<R: Read> BufCodecReader<R> {
    pub fn new(input: BufReader<R>, maybe_text_codec: MaybeTextCodec) -> Self {
        BufCodecReader {
            maybe_text_codec,
            input,
        }
    }
}

impl<R: Read> Iterator for BufCodecReader<R> {
    type Item = Result<StringOrBinary, ShellError>;

    fn next(&mut self) -> Option<Self::Item> {
        let buffer = self.input.fill_buf();
        match buffer {
            Ok(s) => {
                let result = self.maybe_text_codec.decode(s).transpose();

                let buffer_len = s.len();
                self.input.consume(buffer_len);

                result
            }
            Err(e) => Some(Err(ShellError::untagged_runtime_error(e.to_string()))),
        }
    }
}

impl MaybeTextCodec {
    // The constructor takes an Option<&'static Encoding>, because an absence of an encoding indicates that we want BOM sniffing enabled
    pub fn new(encoding: Option<&'static Encoding>) -> Self {
        let decoder = match encoding {
            Some(e) => e.new_decoder_with_bom_removal(),
            None => UTF_8.new_decoder(),
        };
        MaybeTextCodec { decoder }
    }
}

impl Default for MaybeTextCodec {
    fn default() -> Self {
        MaybeTextCodec {
            decoder: UTF_8.new_decoder(),
        }
    }
}

impl MaybeTextCodec {
    pub fn decode(&mut self, src: &[u8]) -> Result<Option<StringOrBinary>, ShellError> {
        if src.is_empty() {
            return Ok(None);
        }

        let mut s = String::with_capacity(OUTPUT_BUFFER_SIZE);

        let (res, _read, replacements) = self.decoder.decode_to_string(src, &mut s, false);

        let result = if replacements {
            // If we had to make replacements when converting to utf8, fall back to binary
            StringOrBinary::Binary(src.to_vec())
        } else {
            // If original buffer size is too small, we continue to allocate new Strings and append
            // them to the result until the input buffer is smaller than the allocated String
            if let CoderResult::OutputFull = res {
                let mut buffer = String::with_capacity(OUTPUT_BUFFER_SIZE);
                loop {
                    let (res, _read, _replacements) =
                        self.decoder
                            .decode_to_string(&src[s.len()..], &mut buffer, false);
                    s.push_str(&buffer);

                    if let CoderResult::InputEmpty = res {
                        break;
                    }

                    buffer.clear();
                }
            }

            StringOrBinary::String(s)
        };

        // src.clear();

        Ok(Some(result))
    }
}
