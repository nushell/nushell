//! Hjson Serialization
//!
//! This module provides for Hjson serialization with the type `Serializer`.

use std::fmt::{Display, LowerExp};
use std::io;
use std::io::{BufRead, BufReader};
use std::num::FpCategory;

use super::error::{Error, ErrorCode, Result};
use serde::ser;

//use super::util::ParseNumber;

use fancy_regex::Regex;

use lazy_static::lazy_static;

/// A structure for serializing Rust values into Hjson.
pub struct Serializer<W, F> {
    writer: W,
    formatter: F,
}

impl<'a, W> Serializer<W, HjsonFormatter<'a>>
where
    W: io::Write,
{
    /// Creates a new Hjson serializer.
    #[inline]
    pub fn new(writer: W) -> Self {
        Serializer::with_formatter(writer, HjsonFormatter::new())
    }

    #[inline]
    pub fn with_indent(writer: W, indent: &'a [u8]) -> Self {
        Serializer::with_formatter(writer, HjsonFormatter::with_indent(indent))
    }
}

impl<W, F> Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    /// Creates a new Hjson visitor whose output will be written to the writer
    /// specified.
    #[inline]
    pub fn with_formatter(writer: W, formatter: F) -> Self {
        Serializer { writer, formatter }
    }

    /// Unwrap the `Writer` from the `Serializer`.
    #[inline]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

#[doc(hidden)]
#[derive(Eq, PartialEq)]
pub enum State {
    Empty,
    First,
    Rest,
}

#[doc(hidden)]
pub struct Compound<'a, W, F> {
    ser: &'a mut Serializer<W, F>,
    state: State,
}

impl<'a, W, F> ser::Serializer for &'a mut Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Compound<'a, W, F>;
    type SerializeTuple = Compound<'a, W, F>;
    type SerializeTupleStruct = Compound<'a, W, F>;
    type SerializeTupleVariant = Compound<'a, W, F>;
    type SerializeMap = Compound<'a, W, F>;
    type SerializeStruct = Compound<'a, W, F>;
    type SerializeStructVariant = Compound<'a, W, F>;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        if value {
            self.writer.write_all(b"true").map_err(From::from)
        } else {
            self.writer.write_all(b"false").map_err(From::from)
        }
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        fmt_f32_or_null(&mut self.writer, if value == -0f32 { 0f32 } else { value })
            .map_err(From::from)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        fmt_f64_or_null(&mut self.writer, if value == -0f64 { 0f64 } else { value })
            .map_err(From::from)
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        escape_char(&mut self.writer, value).map_err(From::from)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        quote_str(&mut self.writer, &mut self.formatter, value).map_err(From::from)
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        let mut seq = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            ser::SerializeSeq::serialize_element(&mut seq, byte)?
        }
        ser::SerializeSeq::end(seq)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        self.writer.write_all(b"null").map_err(From::from)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    /// Serialize newtypes without an object wrapper.
    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        value.serialize(&mut *self)?;
        self.formatter.close(&mut self.writer, b'}')
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let state = if len == Some(0) {
            self.formatter.start_value(&mut self.writer)?;
            self.writer.write_all(b"[]")?;
            State::Empty
        } else {
            self.formatter.open(&mut self.writer, b'[')?;
            State::First
        };
        Ok(Compound { ser: self, state })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let state = if len == Some(0) {
            self.formatter.start_value(&mut self.writer)?;
            self.writer.write_all(b"{}")?;
            State::Empty
        } else {
            self.formatter.open(&mut self.writer, b'{')?;
            State::First
        };
        Ok(Compound { ser: self, state })
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        self.serialize_map(Some(len))
    }
}

impl<'a, W, F> ser::SerializeSeq for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.ser
            .formatter
            .comma(&mut self.ser.writer, self.state == State::First)?;
        self.state = State::Rest;
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok> {
        match self.state {
            State::Empty => Ok(()),
            _ => self.ser.formatter.close(&mut self.ser.writer, b']'),
        }
    }
}

impl<'a, W, F> ser::SerializeTuple for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        match self.state {
            State::Empty => {}
            _ => self.ser.formatter.close(&mut self.ser.writer, b']')?,
        }
        self.ser.formatter.close(&mut self.ser.writer, b'}')
    }
}

impl<'a, W, F> ser::SerializeMap for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.ser
            .formatter
            .comma(&mut self.ser.writer, self.state == State::First)?;
        self.state = State::Rest;

        key.serialize(MapKeySerializer { ser: self.ser })?;

        self.ser.formatter.colon(&mut self.ser.writer)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<Self::Ok> {
        match self.state {
            State::Empty => Ok(()),
            _ => self.ser.formatter.close(&mut self.ser.writer, b'}'),
        }
    }
}

impl<'a, W, F> ser::SerializeStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

impl<'a, W, F> ser::SerializeStructVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        match self.state {
            State::Empty => {}
            _ => self.ser.formatter.close(&mut self.ser.writer, b'}')?,
        }
        self.ser.formatter.close(&mut self.ser.writer, b'}')
    }
}

struct MapKeySerializer<'a, W: 'a, F: 'a> {
    ser: &'a mut Serializer<W, F>,
}

impl<'a, W, F> ser::Serializer for MapKeySerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        escape_key(&mut self.ser.writer, value).map_err(From::from)
    }

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bool(self, _value: bool) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i8(self, _value: i8) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i16(self, _value: i16) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i32(self, _value: i32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i64(self, _value: i64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u8(self, _value: u8) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u16(self, _value: u16) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u32(self, _value: u32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u64(self, _value: u64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_f32(self, _value: f32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_f64(self, _value: f64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_char(self, _value: char) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeStruct> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }
}

/// This trait abstracts away serializing the JSON control characters
pub trait Formatter {
    /// Called when serializing a '{' or '['.
    fn open<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
    where
        W: io::Write;

    /// Called when serializing a ','.
    fn comma<W>(&mut self, writer: &mut W, first: bool) -> Result<()>
    where
        W: io::Write;

    /// Called when serializing a ':'.
    fn colon<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write;

    /// Called when serializing a '}' or ']'.
    fn close<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
    where
        W: io::Write;

    /// Newline with indent.
    fn newline<W>(&mut self, writer: &mut W, add_indent: i32) -> Result<()>
    where
        W: io::Write;

    /// Start a value.
    fn start_value<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write;
}

struct HjsonFormatter<'a> {
    current_indent: usize,
    current_is_array: bool,
    stack: Vec<bool>,
    at_colon: bool,
    indent: &'a [u8],
    braces_same_line: bool,
}

impl<'a> HjsonFormatter<'a> {
    /// Construct a formatter that defaults to using two spaces for indentation.
    pub fn new() -> Self {
        HjsonFormatter::with_indent(b"  ")
    }

    /// Construct a formatter that uses the `indent` string for indentation.
    pub fn with_indent(indent: &'a [u8]) -> Self {
        HjsonFormatter {
            current_indent: 0,
            current_is_array: false,
            stack: Vec::new(),
            at_colon: false,
            indent,
            braces_same_line: false,
        }
    }
}

impl<'a> Formatter for HjsonFormatter<'a> {
    fn open<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
    where
        W: io::Write,
    {
        if self.current_indent > 0 && !self.current_is_array && !self.braces_same_line {
            self.newline(writer, 0)?;
        } else {
            self.start_value(writer)?;
        }
        self.current_indent += 1;
        self.stack.push(self.current_is_array);
        self.current_is_array = ch == b'[';
        writer.write_all(&[ch]).map_err(From::from)
    }

    fn comma<W>(&mut self, writer: &mut W, first: bool) -> Result<()>
    where
        W: io::Write,
    {
        if !first {
            writer.write_all(b",\n")?;
        } else {
            writer.write_all(b"\n")?;
        }
        indent(writer, self.current_indent, self.indent)
    }

    fn colon<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write,
    {
        self.at_colon = !self.braces_same_line;
        writer
            .write_all(if self.braces_same_line { b": " } else { b":" })
            .map_err(From::from)
    }

    fn close<W>(&mut self, writer: &mut W, ch: u8) -> Result<()>
    where
        W: io::Write,
    {
        self.current_indent -= 1;
        self.current_is_array = self.stack.pop().expect("Internal error: json parsing");
        writer.write_all(b"\n")?;
        indent(writer, self.current_indent, self.indent)?;
        writer.write_all(&[ch]).map_err(From::from)
    }

    fn newline<W>(&mut self, writer: &mut W, add_indent: i32) -> Result<()>
    where
        W: io::Write,
    {
        self.at_colon = false;
        writer.write_all(b"\n")?;
        let ii = self.current_indent as i32 + add_indent;
        indent(writer, if ii < 0 { 0 } else { ii as usize }, self.indent)
    }

    fn start_value<W>(&mut self, writer: &mut W) -> Result<()>
    where
        W: io::Write,
    {
        if self.at_colon {
            self.at_colon = false;
            writer.write_all(b" ")?
        }
        Ok(())
    }
}

/// Serializes and escapes a `&[u8]` into a Hjson string.
#[inline]
pub fn escape_bytes<W>(wr: &mut W, bytes: &[u8]) -> Result<()>
where
    W: io::Write,
{
    wr.write_all(b"\"")?;

    let mut start = 0;

    for (i, byte) in bytes.iter().enumerate() {
        let escaped = match *byte {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\x08' => b"\\b",
            b'\x0c' => b"\\f",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            b'\t' => b"\\t",
            _ => {
                continue;
            }
        };

        if start < i {
            wr.write_all(&bytes[start..i])?;
        }

        wr.write_all(escaped)?;

        start = i + 1;
    }

    if start != bytes.len() {
        wr.write_all(&bytes[start..])?;
    }

    wr.write_all(b"\"")?;
    Ok(())
}

/// Serializes and escapes a `&str` into a Hjson string.
#[inline]
pub fn quote_str<W, F>(wr: &mut W, formatter: &mut F, value: &str) -> Result<()>
where
    W: io::Write,
    F: Formatter,
{
    lazy_static! {
        // NEEDS_ESCAPE tests if the string can be written without escapes
        static ref NEEDS_ESCAPE: Regex = Regex::new("[\\\\\"\x00-\x1f\x7f-\u{9f}\u{00ad}\u{0600}-\u{0604}\u{070f}\u{17b4}\u{17b5}\u{200c}-\u{200f}\u{2028}-\u{202f}\u{2060}-\u{206f}\u{feff}\u{fff0}-\u{ffff}]").expect("Internal error: json parsing");
        // NEEDS_QUOTES tests if the string can be written as a quoteless string (includes needsEscape but without \\ and \")
        static ref NEEDS_QUOTES: Regex = Regex::new("^\\s|^\"|^'''|^#|^/\\*|^//|^\\{|^\\}|^\\[|^\\]|^:|^,|\\s$|[\x00-\x1f\x7f-\u{9f}\u{00ad}\u{0600}-\u{0604}\u{070f}\u{17b4}\u{17b5}\u{200c}-\u{200f}\u{2028}-\u{202f}\u{2060}-\u{206f}\u{feff}\u{fff0}-\u{ffff}]").expect("Internal error: json parsing");
        // starts with a keyword and optionally is followed by a comment
        static ref STARTS_WITH_KEYWORD: Regex = Regex::new(r#"^(true|false|null)\s*((,|\]|\}|#|//|/\*).*)?$"#).expect("Internal error: json parsing");
    }

    if value.is_empty() {
        formatter.start_value(wr)?;
        return escape_bytes(wr, value.as_bytes());
    }

    formatter.start_value(wr)?;
    escape_bytes(wr, value.as_bytes())
}

/// Serializes and escapes a `&str` into a Hjson key.
#[inline]
pub fn escape_key<W>(wr: &mut W, value: &str) -> Result<()>
where
    W: io::Write,
{
    lazy_static! {
        static ref NEEDS_ESCAPE_NAME: Regex =
            Regex::new(r#"[,\{\[\}\]\s:#"]|//|/\*|'''|^$"#).expect("Internal error: json parsing");
    }

    escape_bytes(wr, value.as_bytes()).map_err(From::from)
}

#[inline]
fn escape_char<W>(wr: &mut W, value: char) -> Result<()>
where
    W: io::Write,
{
    // FIXME: this allocation is required in order to be compatible with stable
    // rust, which doesn't support encoding a `char` into a stack buffer.
    let mut s = String::new();
    s.push(value);
    escape_bytes(wr, s.as_bytes())
}

fn fmt_f32_or_null<W>(wr: &mut W, value: f32) -> Result<()>
where
    W: io::Write,
{
    match value.classify() {
        FpCategory::Nan | FpCategory::Infinite => wr.write_all(b"null")?,
        _ => wr.write_all(fmt_small(value).as_bytes())?,
    }

    Ok(())
}

fn fmt_f64_or_null<W>(wr: &mut W, value: f64) -> Result<()>
where
    W: io::Write,
{
    match value.classify() {
        FpCategory::Nan | FpCategory::Infinite => wr.write_all(b"null")?,
        _ => wr.write_all(fmt_small(value).as_bytes())?,
    }

    Ok(())
}

fn indent<W>(wr: &mut W, n: usize, s: &[u8]) -> Result<()>
where
    W: io::Write,
{
    for _ in 0..n {
        wr.write_all(s)?;
    }

    Ok(())
}

// format similar to es6
fn fmt_small<N>(value: N) -> String
where
    N: Display + LowerExp,
{
    let f1 = value.to_string();
    let f2 = format!("{:e}", value);
    if f1.len() <= f2.len() + 1 {
        f1
    } else if !f2.contains("e-") {
        f2.replace('e', "e+")
    } else {
        f2
    }
}

/// Encode the specified struct into a Hjson `[u8]` writer.
#[inline]
pub fn to_writer<W, T>(writer: &mut W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    let mut ser = Serializer::new(writer);
    value.serialize(&mut ser)?;
    Ok(())
}

/// Encode the specified struct into a Hjson `[u8]` writer.
#[inline]
pub fn to_writer_with_tab_indentation<W, T>(writer: &mut W, value: &T, tabs: usize) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    let indent_string = "\t".repeat(tabs);
    let mut ser = Serializer::with_indent(writer, indent_string.as_bytes());
    value.serialize(&mut ser)?;
    Ok(())
}

/// Encode the specified struct into a Hjson `[u8]` writer.
#[inline]
pub fn to_writer_with_indent<W, T>(writer: &mut W, value: &T, indent: usize) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    let indent_string = " ".repeat(indent);
    let mut ser = Serializer::with_indent(writer, indent_string.as_bytes());
    value.serialize(&mut ser)?;
    Ok(())
}

/// Encode the specified struct into a Hjson `[u8]` buffer.
#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ser::Serialize,
{
    // We are writing to a Vec, which doesn't fail. So we can ignore
    // the error.
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, value)?;
    Ok(writer)
}

/// Encode the specified struct into a Hjson `[u8]` buffer.
#[inline]
pub fn to_vec_with_tab_indentation<T>(value: &T, tabs: usize) -> Result<Vec<u8>>
where
    T: ser::Serialize,
{
    // We are writing to a Vec, which doesn't fail. So we can ignore
    // the error.
    let mut writer = Vec::with_capacity(128);
    to_writer_with_tab_indentation(&mut writer, value, tabs)?;
    Ok(writer)
}

/// Encode the specified struct into a Hjson `[u8]` buffer.
#[inline]
pub fn to_vec_with_indent<T>(value: &T, indent: usize) -> Result<Vec<u8>>
where
    T: ser::Serialize,
{
    // We are writing to a Vec, which doesn't fail. So we can ignore
    // the error.
    let mut writer = Vec::with_capacity(128);
    to_writer_with_indent(&mut writer, value, indent)?;
    Ok(writer)
}

/// Encode the specified struct into a Hjson `String` buffer.
#[inline]
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ser::Serialize,
{
    let vec = to_vec(value)?;
    let string = String::from_utf8(vec)?;
    Ok(string)
}

/// Encode the specified struct into a Hjson `String` buffer.
#[inline]
pub fn to_string_with_indent<T>(value: &T, indent: usize) -> Result<String>
where
    T: ser::Serialize,
{
    let vec = to_vec_with_indent(value, indent)?;
    let string = String::from_utf8(vec)?;
    Ok(string)
}

/// Encode the specified struct into a Hjson `String` buffer.
#[inline]
pub fn to_string_with_tab_indentation<T>(value: &T, tabs: usize) -> Result<String>
where
    T: ser::Serialize,
{
    let vec = to_vec_with_tab_indentation(value, tabs)?;
    let string = String::from_utf8(vec)?;
    Ok(string)
}

/// Encode the specified struct into a Hjson `String` buffer.
/// And remove all whitespace
#[inline]
pub fn to_string_raw<T>(value: &T) -> Result<String>
where
    T: ser::Serialize,
{
    let vec = to_vec(value)?;
    let string = remove_json_whitespace(vec);
    Ok(string)
}

fn remove_json_whitespace(v: Vec<u8>) -> String {
    let reader = BufReader::new(&v[..]);
    let mut output = String::new();
    for line in reader.lines() {
        match line {
            Ok(line) => output.push_str(line.trim().trim_end()),
            _ => {
                eprintln!("Error removing JSON whitespace");
            }
        }
    }
    output
}
