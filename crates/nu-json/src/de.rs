//! Hjson Deserialization
//!
//! This module provides for Hjson deserialization with the type `Deserializer`.

use super::error::{Error, ErrorCode, Result};
use super::util::StringReader;
use super::util::{Number, ParseNumber};
use serde::de;
use std::{
    char, io,
    io::{BufReader, Read},
    marker::PhantomData,
    str,
};

enum State {
    Normal,
    Root,
    Keyname,
}

/// How deep lists and objects may nest. Each level is a few recursive calls, so deeper input
/// gets an error here instead of overflowing the stack.
const MAX_DEPTH: usize = 1024;

/// A structure that deserializes Hjson into Rust values.
pub struct Deserializer<Iter: Iterator<Item = u8>> {
    rdr: StringReader<Iter>,
    str_buf: Vec<u8>,
    state: State,
    /// How many more lists and objects may open inside the current one.
    remaining_depth: usize,
}

impl<Iter> Deserializer<Iter>
where
    Iter: Iterator<Item = u8>,
{
    /// Creates the Hjson parser from an `std::iter::Iterator`.
    #[inline]
    pub fn new(rdr: Iter) -> Deserializer<Iter> {
        Deserializer {
            rdr: StringReader::new(rdr),
            str_buf: Vec::with_capacity(128),
            state: State::Normal,
            remaining_depth: MAX_DEPTH,
        }
    }

    /// Creates the Hjson parser from an `std::iter::Iterator`.
    #[inline]
    pub fn new_for_root(rdr: Iter) -> Deserializer<Iter> {
        let mut res = Deserializer::new(rdr);
        res.state = State::Root;
        res
    }

    /// The `Deserializer::end` method should be called after a value has been fully deserialized.
    /// This allows the `Deserializer` to validate that the input stream is at the end or that it
    /// only has trailing whitespace.
    #[inline]
    pub fn end(&mut self) -> Result<()> {
        self.rdr.parse_whitespace()?;
        if self.rdr.eof()? {
            Ok(())
        } else {
            Err(self.rdr.error(ErrorCode::TrailingCharacters))
        }
    }

    fn is_punctuator_char(&self, ch: u8) -> bool {
        matches!(ch, b'{' | b'}' | b'[' | b']' | b',' | b':')
    }

    fn parse_keyname<'de, V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // quotes for keys are optional in Hjson
        // unless they include {}[],: or whitespace.
        // assume whitespace was already eaten

        self.str_buf.clear();

        let mut space: Option<usize> = None;
        loop {
            let ch = self.rdr.next_char_or_null()?;

            if ch == b':' {
                if self.str_buf.is_empty() {
                    return Err(self.rdr.error(ErrorCode::Custom(
                        "Found ':' but no key name (for an empty key name use quotes)".to_string(),
                    )));
                } else if space.is_some()
                    && space.expect("Internal error: json parsing") != self.str_buf.len()
                {
                    return Err(self.rdr.error(ErrorCode::Custom(
                        "Found whitespace in your key name (use quotes to include)".to_string(),
                    )));
                }
                self.rdr.uneat_char(ch);
                let s = str::from_utf8(&self.str_buf).expect("Internal error: json parsing");
                return visitor.visit_str(s);
            } else if ch <= b' ' {
                if ch == 0 {
                    return Err(self.rdr.error(ErrorCode::EofWhileParsingObject));
                } else if space.is_none() {
                    space = Some(self.str_buf.len());
                }
            } else if self.is_punctuator_char(ch) {
                return Err(self.rdr.error(ErrorCode::Custom("Found a punctuator where a key name was expected (check your syntax or use quotes if the key name includes {}[],: or whitespace)".to_string())));
            } else {
                self.str_buf.push(ch);
            }
        }
    }

    fn parse_value<'de, V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.rdr.parse_whitespace()?;

        if self.rdr.eof()? {
            return Err(self.rdr.error(ErrorCode::EofWhileParsingValue));
        }

        match self.state {
            State::Keyname => {
                self.state = State::Normal;
                return self.parse_keyname(visitor);
            }
            State::Root => {
                self.state = State::Normal;
                return self.visit_map(true, visitor);
            }
            _ => {}
        }

        match self.rdr.peek_or_null()? {
            /*
            b'-' => {
                self.rdr.eat_char();
                self.parse_integer(false, visitor)
            }
            b'0' ... b'9' => {
                self.parse_integer(true, visitor)
            }
            */
            b'"' => {
                self.rdr.eat_char();
                self.parse_string()?;
                let s = str::from_utf8(&self.str_buf).expect("Internal error: json parsing");
                visitor.visit_str(s)
            }
            b'[' => self.nested(|de| {
                de.rdr.eat_char();
                let ret = visitor.visit_seq(SeqVisitor::new(de))?;
                de.rdr.parse_whitespace()?;
                match de.rdr.next_char()? {
                    Some(b']') => Ok(ret),
                    Some(_) => Err(de.rdr.error(ErrorCode::TrailingCharacters)),
                    None => Err(de.rdr.error(ErrorCode::EofWhileParsingList)),
                }
            }),
            b'{' => self.nested(|de| {
                de.rdr.eat_char();
                de.visit_map(false, visitor)
            }),
            b'\x00' => Err(self.rdr.error(ErrorCode::ExpectedSomeValue)),
            _ => self.parse_tfnns(visitor),
        }
    }

    /// Runs `parse` on a list or object one level deeper, or fails past [`MAX_DEPTH`].
    fn nested<T>(&mut self, parse: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.remaining_depth == 0 {
            return Err(self.rdr.error(ErrorCode::Custom(format!(
                "Found lists and objects nested more than {MAX_DEPTH} deep"
            ))));
        }
        self.remaining_depth -= 1;
        let result = parse(self);
        self.remaining_depth += 1;
        result
    }

    fn visit_map<'de, V>(&mut self, root: bool, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let ret = visitor.visit_map(MapVisitor::new(self, root))?;
        self.rdr.parse_whitespace()?;
        match self.rdr.next_char()? {
            Some(b'}') => {
                if !root {
                    Ok(ret)
                } else {
                    Err(self.rdr.error(ErrorCode::TrailingCharacters))
                } // todo
            }
            Some(_) => Err(self.rdr.error(ErrorCode::TrailingCharacters)),
            None => {
                if root {
                    Ok(ret)
                } else {
                    Err(self.rdr.error(ErrorCode::EofWhileParsingObject))
                }
            }
        }
    }

    fn parse_ident(&mut self, ident: &[u8]) -> Result<()> {
        for c in ident {
            if Some(*c) != self.rdr.next_char()? {
                return Err(self.rdr.error(ErrorCode::ExpectedSomeIdent));
            }
        }

        Ok(())
    }

    fn parse_tfnns<'de, V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        // Hjson strings can be quoteless
        // returns string, true, false, or null.
        self.str_buf.clear();

        let first = self.rdr.peek()?.expect("Internal error: json parsing");

        if self.is_punctuator_char(first) {
            return Err(self.rdr.error(ErrorCode::PunctuatorInQlString));
        }

        loop {
            let ch = self.rdr.next_char_or_null()?;

            let is_eol = ch == b'\r' || ch == b'\n' || ch == b'\x00';
            let is_comment = ch == b'#'
                || if ch == b'/' {
                    let next = self.rdr.peek_or_null()?;
                    next == b'/' || next == b'*'
                } else {
                    false
                };
            if is_eol || is_comment || ch == b',' || ch == b'}' || ch == b']' {
                let chf = self.str_buf[0];
                match chf {
                    b'f' => {
                        if str::from_utf8(&self.str_buf)
                            .expect("Internal error: json parsing")
                            .trim()
                            == "false"
                        {
                            self.rdr.uneat_char(ch);
                            return visitor.visit_bool(false);
                        }
                    }
                    b'n' => {
                        if str::from_utf8(&self.str_buf)
                            .expect("Internal error: json parsing")
                            .trim()
                            == "null"
                        {
                            self.rdr.uneat_char(ch);
                            return visitor.visit_unit();
                        }
                    }
                    b't' => {
                        if str::from_utf8(&self.str_buf)
                            .expect("Internal error: json parsing")
                            .trim()
                            == "true"
                        {
                            self.rdr.uneat_char(ch);
                            return visitor.visit_bool(true);
                        }
                    }
                    _ => {
                        if chf == b'-' || chf.is_ascii_digit() {
                            let mut parser = ParseNumber::new(self.str_buf.iter().copied());
                            match parser.parse(false) {
                                Ok(Number::F64(v)) => {
                                    self.rdr.uneat_char(ch);
                                    return visitor.visit_f64(v);
                                }
                                Ok(Number::U64(v)) => {
                                    self.rdr.uneat_char(ch);
                                    return visitor.visit_u64(v);
                                }
                                Ok(Number::I64(v)) => {
                                    self.rdr.uneat_char(ch);
                                    return visitor.visit_i64(v);
                                }
                                Err(_) => {} // not a number, continue
                            }
                        }
                    }
                }
                if is_eol {
                    // remove any whitespace at the end (ignored in quoteless strings)
                    return visitor.visit_str(
                        str::from_utf8(&self.str_buf)
                            .expect("Internal error: json parsing")
                            .trim(),
                    );
                }
            }
            self.str_buf.push(ch);

            if self.str_buf == b"'''" {
                return self.parse_ml_string(visitor);
            }
        }
    }

    fn decode_hex_escape(&mut self) -> Result<u16> {
        let mut i = 0;
        let mut n = 0u16;
        while i < 4 && !self.rdr.eof()? {
            n = match self.rdr.next_char_or_null()? {
                c @ b'0'..=b'9' => n * 16_u16 + ((c as u16) - (b'0' as u16)),
                b'a' | b'A' => n * 16_u16 + 10_u16,
                b'b' | b'B' => n * 16_u16 + 11_u16,
                b'c' | b'C' => n * 16_u16 + 12_u16,
                b'd' | b'D' => n * 16_u16 + 13_u16,
                b'e' | b'E' => n * 16_u16 + 14_u16,
                b'f' | b'F' => n * 16_u16 + 15_u16,
                _ => {
                    return Err(self.rdr.error(ErrorCode::InvalidEscape));
                }
            };

            i += 1;
        }

        // Error out if we didn't parse 4 digits.
        if i != 4 {
            return Err(self.rdr.error(ErrorCode::InvalidEscape));
        }

        Ok(n)
    }

    fn ml_skip_white(&mut self) -> Result<bool> {
        match self.rdr.peek_or_null()? {
            b' ' | b'\t' | b'\r' => {
                self.rdr.eat_char();
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn ml_skip_indent(&mut self, indent: usize) -> Result<()> {
        let mut skip = indent;
        while self.ml_skip_white()? && skip > 0 {
            skip -= 1;
        }
        Ok(())
    }

    fn parse_ml_string<'de, V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.str_buf.clear();

        // Parse a multiline string value.
        let mut triple = 0;

        // `col` is the column of the third quote, so the opening ''' starts `col - 3` bytes into
        // its line. `ml_skip_indent` skips one byte more than it is asked to, hence the `- 4`.
        // When ''' starts the line there is nothing to subtract from. Release builds have always
        // wrapped to `usize::MAX` there, which strips all leading whitespace, so keep that.
        let (_, col) = self.rdr.pos();
        let indent = col.checked_sub(4).unwrap_or(usize::MAX);

        // skip white/to (newline)
        while self.ml_skip_white()? {}
        if self.rdr.peek_or_null()? == b'\n' {
            self.rdr.eat_char();
            self.ml_skip_indent(indent)?;
        }

        // When parsing multiline string values, we must look for ' characters.
        loop {
            if self.rdr.eof()? {
                return Err(self.rdr.error(ErrorCode::EofWhileParsingString));
            } // todo error("Bad multiline string");
            let ch = self.rdr.next_char_or_null()?;

            if ch == b'\'' {
                triple += 1;
                if triple == 3 {
                    if self.str_buf.last() == Some(&b'\n') {
                        self.str_buf.pop();
                    }
                    let res = str::from_utf8(&self.str_buf).expect("Internal error: json parsing");
                    //todo if (self.str_buf.slice(-1) === '\n') self.str_buf=self.str_buf.slice(0, -1); // remove last EOL
                    return visitor.visit_str(res);
                } else {
                    continue;
                }
            }

            while triple > 0 {
                self.str_buf.push(b'\'');
                triple -= 1;
            }

            if ch != b'\r' {
                self.str_buf.push(ch);
            }
            if ch == b'\n' {
                self.ml_skip_indent(indent)?;
            }
        }
    }

    fn parse_string(&mut self) -> Result<()> {
        self.str_buf.clear();

        loop {
            let ch = match self.rdr.next_char()? {
                Some(ch) => ch,
                None => {
                    return Err(self.rdr.error(ErrorCode::EofWhileParsingString));
                }
            };

            match ch {
                b'"' => {
                    return Ok(());
                }
                b'\\' => {
                    let ch = match self.rdr.next_char()? {
                        Some(ch) => ch,
                        None => {
                            return Err(self.rdr.error(ErrorCode::EofWhileParsingString));
                        }
                    };

                    match ch {
                        b'"' => self.str_buf.push(b'"'),
                        b'\\' => self.str_buf.push(b'\\'),
                        b'/' => self.str_buf.push(b'/'),
                        b'b' => self.str_buf.push(b'\x08'),
                        b'f' => self.str_buf.push(b'\x0c'),
                        b'n' => self.str_buf.push(b'\n'),
                        b'r' => self.str_buf.push(b'\r'),
                        b't' => self.str_buf.push(b'\t'),
                        b'u' => {
                            let c = match self.decode_hex_escape()? {
                                0xDC00..=0xDFFF => {
                                    return Err(self
                                        .rdr
                                        .error(ErrorCode::LoneLeadingSurrogateInHexEscape));
                                }

                                // Non-BMP characters are encoded as a sequence of
                                // two hex escapes, representing UTF-16 surrogates.
                                n1 @ 0xD800..=0xDBFF => {
                                    match (self.rdr.next_char()?, self.rdr.next_char()?) {
                                        (Some(b'\\'), Some(b'u')) => (),
                                        _ => {
                                            return Err(self
                                                .rdr
                                                .error(ErrorCode::UnexpectedEndOfHexEscape));
                                        }
                                    }

                                    let n2 = self.decode_hex_escape()?;

                                    if !(0xDC00..=0xDFFF).contains(&n2) {
                                        return Err(self
                                            .rdr
                                            .error(ErrorCode::LoneLeadingSurrogateInHexEscape));
                                    }

                                    let n = ((((n1 - 0xD800) as u32) << 10) | (n2 - 0xDC00) as u32)
                                        + 0x1_0000;

                                    match char::from_u32(n) {
                                        Some(c) => c,
                                        None => {
                                            return Err(self
                                                .rdr
                                                .error(ErrorCode::InvalidUnicodeCodePoint));
                                        }
                                    }
                                }

                                n => match char::from_u32(n as u32) {
                                    Some(c) => c,
                                    None => {
                                        return Err(self
                                            .rdr
                                            .error(ErrorCode::InvalidUnicodeCodePoint));
                                    }
                                },
                            };

                            self.str_buf.extend(c.encode_utf8(&mut [0; 4]).as_bytes());
                        }
                        _ => {
                            return Err(self.rdr.error(ErrorCode::InvalidEscape));
                        }
                    }
                }
                ch => {
                    self.str_buf.push(ch);
                }
            }
        }
    }

    fn parse_object_colon(&mut self) -> Result<()> {
        self.rdr.parse_whitespace()?;

        match self.rdr.next_char()? {
            Some(b':') => Ok(()),
            Some(_) => Err(self.rdr.error(ErrorCode::ExpectedColon)),
            None => Err(self.rdr.error(ErrorCode::EofWhileParsingObject)),
        }
    }
}

impl<'de, Iter> de::Deserializer<'de> for &mut Deserializer<Iter>
where
    Iter: Iterator<Item = u8>,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if let State::Root = self.state {}

        self.parse_value(visitor)
    }

    /// Parses a `null` as a None, and any other values as a `Some(...)`.
    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.rdr.parse_whitespace()?;

        match self.rdr.peek_or_null()? {
            b'n' => {
                self.rdr.eat_char();
                self.parse_ident(b"ull")?;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    /// Parses a newtype struct as the underlying value.
    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct seq tuple map
        tuple_struct struct enum identifier ignored_any
    }
}

struct SeqVisitor<'a, Iter: 'a + Iterator<Item = u8>> {
    de: &'a mut Deserializer<Iter>,
}

impl<'a, Iter: Iterator<Item = u8>> SeqVisitor<'a, Iter> {
    fn new(de: &'a mut Deserializer<Iter>) -> Self {
        SeqVisitor { de }
    }
}

impl<'de, Iter> de::SeqAccess<'de> for SeqVisitor<'_, Iter>
where
    Iter: Iterator<Item = u8>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.de.rdr.parse_whitespace()?;

        match self.de.rdr.peek()? {
            Some(b']') => {
                return Ok(None);
            }
            Some(_) => {}
            None => {
                return Err(self.de.rdr.error(ErrorCode::EofWhileParsingList));
            }
        }

        let value = seed.deserialize(&mut *self.de)?;

        // in Hjson the comma is optional and trailing commas are allowed
        self.de.rdr.parse_whitespace()?;
        if self.de.rdr.peek()? == Some(b',') {
            self.de.rdr.eat_char();
            self.de.rdr.parse_whitespace()?;
        }

        Ok(Some(value))
    }
}

struct MapVisitor<'a, Iter: 'a + Iterator<Item = u8>> {
    de: &'a mut Deserializer<Iter>,
    first: bool,
    root: bool,
}

impl<'a, Iter: Iterator<Item = u8>> MapVisitor<'a, Iter> {
    fn new(de: &'a mut Deserializer<Iter>, root: bool) -> Self {
        MapVisitor {
            de,
            first: true,
            root,
        }
    }
}

impl<'de, Iter> de::MapAccess<'de> for MapVisitor<'_, Iter>
where
    Iter: Iterator<Item = u8>,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.de.rdr.parse_whitespace()?;

        if self.first {
            self.first = false;
        } else if self.de.rdr.peek()? == Some(b',') {
            // in Hjson the comma is optional and trailing commas are allowed
            self.de.rdr.eat_char();
            self.de.rdr.parse_whitespace()?;
        }

        match self.de.rdr.peek()? {
            Some(b'}') => return Ok(None), // handled later for root
            Some(_) => {}
            None => {
                if self.root {
                    return Ok(None);
                } else {
                    return Err(self.de.rdr.error(ErrorCode::EofWhileParsingObject));
                }
            }
        }

        match self.de.rdr.peek()? {
            Some(ch) => {
                self.de.state = if ch == b'"' {
                    State::Normal
                } else {
                    State::Keyname
                };
                Ok(Some(seed.deserialize(&mut *self.de)?))
            }
            None => Err(self.de.rdr.error(ErrorCode::EofWhileParsingValue)),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.de.parse_object_colon()?;

        seed.deserialize(&mut *self.de)
    }
}

impl<'de, Iter> de::VariantAccess<'de> for &mut Deserializer<Iter>
where
    Iter: Iterator<Item = u8>,
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        de::Deserialize::deserialize(self)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_any(self, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        de::Deserializer::deserialize_any(self, visitor)
    }
}

//////////////////////////////////////////////////////////////////////////////

/// Iterator that deserializes a stream into multiple Hjson values.
pub struct StreamDeserializer<T, Iter>
where
    Iter: Iterator<Item = u8>,
    T: de::DeserializeOwned,
{
    deser: Deserializer<Iter>,
    _marker: PhantomData<T>,
}

impl<T, Iter> StreamDeserializer<T, Iter>
where
    Iter: Iterator<Item = u8>,
    T: de::DeserializeOwned,
{
    /// Returns an `Iterator` of decoded Hjson values from an iterator over
    /// `Iterator<Item=u8>`.
    pub fn new(iter: Iter) -> StreamDeserializer<T, Iter> {
        StreamDeserializer {
            deser: Deserializer::new(iter),
            _marker: PhantomData,
        }
    }
}

impl<T, Iter> Iterator for StreamDeserializer<T, Iter>
where
    Iter: Iterator<Item = u8>,
    T: de::DeserializeOwned,
{
    type Item = Result<T>;

    fn next(&mut self) -> Option<Result<T>> {
        // skip whitespaces, if any
        // this helps with trailing whitespaces, since whitespaces between
        // values are handled for us.
        if let Err(e) = self.deser.rdr.parse_whitespace() {
            return Some(Err(e));
        };

        match self.deser.rdr.eof() {
            Ok(true) => None,
            Ok(false) => match de::Deserialize::deserialize(&mut self.deser) {
                Ok(v) => Some(Ok(v)),
                Err(e) => Some(Err(e)),
            },
            Err(e) => Some(Err(e)),
        }
    }
}

//////////////////////////////////////////////////////////////////////////////

/// Decodes a Hjson value from an iterator over an iterator
/// `Iterator<Item=u8>`.
pub fn from_iter<I, T>(iter: I) -> Result<T>
where
    I: Iterator<Item = io::Result<u8>>,
    T: de::DeserializeOwned,
{
    let bytes = iter.collect::<io::Result<Vec<_>>>().map_err(Error::Io)?;
    from_slice(&bytes)
}

/// Decodes a Hjson value from a `std::io::Read`.
pub fn from_reader<R, T>(rdr: R) -> Result<T>
where
    R: io::Read,
    T: de::DeserializeOwned,
{
    // Use a buffered reader so that calling `.bytes()` is efficient and
    // doesn't trigger clippy's `unbuffered_bytes` lint when the input
    // comes from a non-memory source.
    from_iter(BufReader::new(rdr).bytes())
}

/// Decodes a Hjson value from a byte slice `&[u8]`.
pub fn from_slice<T>(v: &[u8]) -> Result<T>
where
    T: de::DeserializeOwned,
{
    from_slice_seed(v, PhantomData::<T>)
}

/// Decodes a Hjson value from a byte slice with `seed`, which decides what the value is built
/// into (see [`de::DeserializeSeed`]).
///
/// Like [`from_slice`], this first reads the input as a root object without braces (legacy Hjson)
/// and, if that fails, as a plain value. Either way the whole input must be consumed.
pub fn from_slice_seed<S, T>(v: &[u8], seed: S) -> Result<T>
where
    S: for<'de> de::DeserializeSeed<'de, Value = T> + Clone,
{
    let mut de = Deserializer::new_for_root(v.iter().copied());
    seed.clone()
        .deserialize(&mut de)
        .and_then(|x| de.end().map(|()| x))
        .or_else(|_| {
            let mut de2 = Deserializer::new(v.iter().copied());
            seed.deserialize(&mut de2)
                .and_then(|x| de2.end().map(|()| x))
        })
}

/// Decodes a Hjson value from a `&str`.
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: de::DeserializeOwned,
{
    from_slice(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn unclosed_block_comment_is_an_error() {
        for input in ["[1, 2] /* x", "[1, 2] /* x *", "[1, /* x", "/*/"] {
            let err = from_str::<Value>(input).expect_err(input);
            assert!(
                err.to_string().contains("no closing */"),
                "{input:?}: {err}"
            );
        }
        let expected = from_str::<Value>("[1, 2]").expect("valid input parses");
        for input in [
            "[1, 2] /* x */",
            "[1, /**/ 2]",
            "/***/ [1, 2]",
            "[1, 2 /* * / */]",
        ] {
            assert_eq!(
                from_str::<Value>(input).expect(input),
                expected,
                "{input:?}"
            );
        }
    }

    #[test]
    fn multiline_string_indent() {
        // The column of the opening ''' sets how much indent each line loses.
        let value: Value = from_str("[\n  '''\n    abc\n      def\n  '''\n]").expect("parses");
        assert_eq!(
            value,
            Value::Array(vec![Value::String("  abc\n    def".into())])
        );
        // At the start of a line it strips all leading whitespace, as release builds always did.
        let value: Value = from_str("'''\n  abc\n    def\n'''").expect("parses");
        assert_eq!(value, Value::String("abc\ndef".into()));
    }

    #[test]
    fn nesting_past_max_depth_is_an_error() {
        // Debug builds use far more stack per level than release builds, and the default test
        // thread is too small for `MAX_DEPTH` levels there.
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let nest = |open: &str, inner: &str, close: &str, depth: usize| {
                    format!("{}{inner}{}", open.repeat(depth), close.repeat(depth))
                };
                for (open, inner, close) in [("[", "", "]"), (r#"{"a":"#, "1", "}")] {
                    let input = nest(open, inner, close, MAX_DEPTH);
                    assert!(from_str::<Value>(&input).is_ok(), "{open} at the limit");
                    let input = nest(open, inner, close, MAX_DEPTH + 1);
                    let err = from_str::<Value>(&input).expect_err("past the limit");
                    assert!(
                        err.to_string().contains("nested more than"),
                        "{open}: {err}"
                    );
                }
            })
            .expect("test thread starts")
            .join()
            .expect("test thread does not panic");
    }
}
