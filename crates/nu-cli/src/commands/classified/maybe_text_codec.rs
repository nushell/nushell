use bytes::{BufMut, Bytes, BytesMut};

use nu_errors::ShellError;

extern crate encoding_rs;
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

impl futures_codec::Encoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = std::io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            StringOrBinary::String(s) => {
                dst.reserve(s.len());
                dst.put(s.as_bytes());
                Ok(())
            }
            StringOrBinary::Binary(b) => {
                dst.reserve(b.len());
                dst.put(Bytes::from(b));
                Ok(())
            }
        }
    }
}

impl futures_codec::Decoder for MaybeTextCodec {
    type Item = StringOrBinary;
    type Error = ShellError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
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

        src.clear();

        Ok(Some(result))
    }
}

#[cfg(test)]
mod tests {
    use super::{MaybeTextCodec, StringOrBinary};
    use bytes::BytesMut;
    use futures_codec::Decoder;

    // TODO: Write some more tests

    #[test]
    fn should_consume_all_bytes_from_source_when_temporary_buffer_overflows() {
        let mut maybe_text = MaybeTextCodec::new(None);
        let mut bytes = BytesMut::from("0123456789");

        let text = maybe_text.decode(&mut bytes);

        assert_eq!(
            Ok(Some(StringOrBinary::String("0123456789".to_string()))),
            text
        );
        assert!(bytes.is_empty());
    }
}
