use bytes::{BufMut, Bytes, BytesMut};

use nu_errors::ShellError;

extern crate encoding_rs;
use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8};

const OUTPUT_BUFFER_SIZE: usize = 8192;

pub enum StringOrBinary {
    String(String),
    Binary(Vec<u8>),
}

#[derive(Debug)]
pub enum EncodingGuess {
    Unknown,
    Known, // An encoding that encoding_rs can determine via BOM sniffing
    Binary,
}

pub struct MaybeTextCodec {
    guess: EncodingGuess,
    decoder: Decoder,
}

impl MaybeTextCodec {
    // The constructor takes an Option<&'static Encoding>, because an absence of an encoding indicates that we want BOM sniffing enabled
    pub fn new(encoding: Option<&'static Encoding>) -> Self {
        let (decoder, guess) = match encoding {
            Some(e) => (e.new_decoder_with_bom_removal(), EncodingGuess::Known),
            None => (UTF_8.new_decoder(), EncodingGuess::Unknown),
        };
        MaybeTextCodec { guess, decoder }
    }
}

impl Default for MaybeTextCodec {
    // The default MaybeTextCodec uses a UTF_8 decoder
    fn default() -> Self {
        MaybeTextCodec {
            guess: EncodingGuess::Unknown,
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
        // The encoding has not been specified or guessed yet, try to figure out what the encoding is
        if let EncodingGuess::Unknown = self.guess {
            self.guess = guess_encoding(src);
        }

        // The encoding is binary, so just spit out the binary
        if let EncodingGuess::Binary = self.guess {
            return Ok(Some(StringOrBinary::Binary(src.to_vec())));
        }

        let (res, read, _replacements) = self.decoder.decode_to_string(src, &mut s, false);
        match res {
            CoderResult::InputEmpty => {
                src.clear();
                Ok(Some(StringOrBinary::String(s)))
            }
            CoderResult::OutputFull => {
                // If the original buffer size is too small,
                // We continue to allocate new Strings and append them to the result until the input buffer is smaller than the allocated String
                // TODO: This is pretty stupid to be allocating String like this right? Best to use Vec?
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

// Function that reads the first couple bytes to determine what type of encoding we are dealing with
// Note that it's not necessary to specify which exact encoding it is because encoding_rs already does BOM sniffing
// It does this the process of elimination e.g. not utf-8 AND not utf-16 AND ...
pub fn guess_encoding(first_bytes: &[u8]) -> EncodingGuess {
    let (b0, b1) = (first_bytes.get(0), first_bytes.get(1));
    // I guess if there is 0 or 1 byte than it's probably binary
    if b0.is_none() || b1.is_none() {
        return EncodingGuess::Binary;
    };
    const EXPECT_MESSAGE: &'static str = "Expected a byte";
    // Now we will do some BOM sniffing to determine if we are NOT dealing with binary
    let (x, y, oz): (&u8, &u8, Option<&u8>) = (
        b0.expect(EXPECT_MESSAGE),
        b1.expect(EXPECT_MESSAGE),
        first_bytes.get(2),
    );

    // From https://en.wikipedia.org/wiki/Byte_order_mark

    // UTF-8
    if *x == 0xef && *y == 0xbb && oz.is_some() && *oz.expect(EXPECT_MESSAGE) == 0xbf {
        return EncodingGuess::Known;
    }

    // UTF-16 Little Endian
    if *x == 0xff && *y == 0xfe {
        return EncodingGuess::Known;
    }
    // UTF-16 Big Endian
    if *x == 0xfe && *y == 0xff {
        return EncodingGuess::Known;
    }
    // We couldn't figure it out from the BOM, so let's just let encoding_rs figure it out OR fallback to Binary where necessary
    EncodingGuess::Unknown

    // TODO: Other BOMs? UTF-32 etc... Although note that encoding_rs only supports sniffing for utf-8, utf-16le, utf-16be
}
