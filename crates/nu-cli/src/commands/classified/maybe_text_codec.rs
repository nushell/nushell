use bytes::{BufMut, Bytes, BytesMut};
use log::trace;

use nu_errors::ShellError;

extern crate encoding_rs;
use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};

const OUTPUT_BUFFER_SIZE: usize = 8192;

pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}

enum EncodingGuess {
    NoGuess,
    Specified,
    Binary,
}

pub struct MaybeTextCodec {
    guess: EncodingGuess,
    decoder: Decoder,
}

impl MaybeTextCodec {
    pub fn new(encoding: Option<&'static Encoding>) -> Self {
        let (decoder, guess) = match encoding {
            Some(e) => (e.new_decoder_with_bom_removal(), EncodingGuess::Specified),
            None => (UTF_8.new_decoder(), EncodingGuess::NoGuess),
        };
        MaybeTextCodec { guess, decoder }
    }
}

impl Default for MaybeTextCodec {
    // The default MaybeTextCodec uses a UTF_8 decoder
    fn default() -> Self {
        MaybeTextCodec {
            guess: EncodingGuess::NoGuess,
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
        // TODO: Do something different if nothing has been guessed
        match self.guess {
            // The encoding has not been specified or guess yet, try to figure out what the encoding is
            EncodingGuess::NoGuess => {}
            // The encoding is binary, so just spit out the binary
            EncodingGuess::Binary => {}
            // The user already specified a encoding so just use that without BOM sniffing
            EncodingGuess::Specified => {}
        }
        // TODO: If raw passed I need to disable BOM sniffing
        let (res, read, _replacements) = self.decoder.decode_to_string(src, &mut s, false);
        match res {
            CoderResult::InputEmpty => {
                src.clear();
                Ok(Some(StringOrBinary::String(s)))
            }
            CoderResult::OutputFull => {
                // If the original buffer size is too small,
                // We continue to allocate new Strings and append them to the result until the input buffer is smaller than the allocated String
                // TODO: This is pretty stupid to be allocating String like this right?
                let mut starting_index = read;
                loop {
                    let mut more = String::with_capacity(OUTPUT_BUFFER_SIZE);
                    let (res, read, _replacements) =
                        self.decoder
                            .decode_to_string(&src[starting_index..], &mut more, false);
                    s.push_str(&more);
                    // Our input buffer is smaller than out allocated String, we can stop now
                    if let CoderResult::InputEmpty = res {
                        break;
                    }
                    starting_index += read;
                }
                src.clear();
                Ok(Some(StringOrBinary::String(s)))
            }
        }
    }
}
