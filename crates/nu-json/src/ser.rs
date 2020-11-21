//! Hjson Serialization
//!
//! This module provides for Hjson serialization with the type `Serializer`.

use std::fmt::{Display, LowerExp};
use std::io;
use std::num::FpCategory;

use super::error::{Error, ErrorCode, Result};
use serde::ser;

use super::util::ParseNumber;

use regex::Regex;

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

impl<W, F> ser::Serializer for Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Error = Error;

    type SeqState = State;
    type TupleState = State;
    type TupleStructState = State;
    type TupleVariantState = State;
    type MapState = State;
    type StructState = State;
    type StructVariantState = State;

    #[inline]
    fn serialize_bool(&mut self, value: bool) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        if value {
            self.writer.write_all(b"true").map_err(From::from)
        } else {
            self.writer.write_all(b"false").map_err(From::from)
        }
    }

    #[inline]
    fn serialize_isize(&mut self, value: isize) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i8(&mut self, value: i8) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i16(&mut self, value: i16) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i32(&mut self, value: i32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_i64(&mut self, value: i64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_usize(&mut self, value: usize) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u8(&mut self, value: u8) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u16(&mut self, value: u16) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u32(&mut self, value: u32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_u64(&mut self, value: u64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        write!(&mut self.writer, "{}", value).map_err(From::from)
    }

    #[inline]
    fn serialize_f32(&mut self, value: f32) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        fmt_f32_or_null(&mut self.writer, if value == -0f32 { 0f32 } else { value })
            .map_err(From::from)
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        fmt_f64_or_null(&mut self.writer, if value == -0f64 { 0f64 } else { value })
            .map_err(From::from)
    }

    #[inline]
    fn serialize_char(&mut self, value: char) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        escape_char(&mut self.writer, value).map_err(From::from)
    }

    #[inline]
    fn serialize_str(&mut self, value: &str) -> Result<()> {
        quote_str(&mut self.writer, &mut self.formatter, value).map_err(From::from)
    }

    #[inline]
    fn serialize_bytes(&mut self, value: &[u8]) -> Result<()> {
        let mut state = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            self.serialize_seq_elt(&mut state, byte)?;
        }
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_unit(&mut self) -> Result<()> {
        self.formatter.start_value(&mut self.writer)?;
        self.writer.write_all(b"null").map_err(From::from)
    }

    #[inline]
    fn serialize_unit_struct(&mut self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    /// Serialize newtypes without an object wrapper.
    #[inline]
    fn serialize_newtype_struct<T>(&mut self, _name: &'static str, value: T) -> Result<()>
    where
        T: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        value: T,
    ) -> Result<()>
    where
        T: ser::Serialize,
    {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        value.serialize(self)?;
        self.formatter.close(&mut self.writer, b'}')
    }

    #[inline]
    fn serialize_none(&mut self) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(&mut self, value: V) -> Result<()>
    where
        V: ser::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(&mut self, len: Option<usize>) -> Result<State> {
        if len == Some(0) {
            self.formatter.start_value(&mut self.writer)?;
            self.writer.write_all(b"[]")?;
            Ok(State::Empty)
        } else {
            self.formatter.open(&mut self.writer, b'[')?;
            Ok(State::First)
        }
    }

    #[inline]
    fn serialize_seq_elt<T: ser::Serialize>(&mut self, state: &mut State, value: T) -> Result<()>
    where
        T: ser::Serialize,
    {
        self.formatter
            .comma(&mut self.writer, *state == State::First)?;
        *state = State::Rest;
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq_end(&mut self, state: State) -> Result<()> {
        match state {
            State::Empty => Ok(()),
            _ => self.formatter.close(&mut self.writer, b']'),
        }
    }

    #[inline]
    fn serialize_seq_fixed_size(&mut self, size: usize) -> Result<State> {
        self.serialize_seq(Some(size))
    }

    #[inline]
    fn serialize_tuple(&mut self, len: usize) -> Result<State> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_elt<T: ser::Serialize>(
        &mut self,
        state: &mut State,
        value: T,
    ) -> Result<()> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_end(&mut self, state: State) -> Result<()> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_struct(&mut self, _name: &'static str, len: usize) -> Result<State> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct_elt<T: ser::Serialize>(
        &mut self,
        state: &mut State,
        value: T,
    ) -> Result<()> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_struct_end(&mut self, state: State) -> Result<()> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<State> {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant_elt<T: ser::Serialize>(
        &mut self,
        state: &mut State,
        value: T,
    ) -> Result<()> {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_variant_end(&mut self, state: State) -> Result<()> {
        self.serialize_seq_end(state)?;
        self.formatter.close(&mut self.writer, b'}')
    }

    #[inline]
    fn serialize_map(&mut self, len: Option<usize>) -> Result<State> {
        if len == Some(0) {
            self.formatter.start_value(&mut self.writer)?;
            self.writer.write_all(b"{}")?;
            Ok(State::Empty)
        } else {
            self.formatter.open(&mut self.writer, b'{')?;
            Ok(State::First)
        }
    }

    #[inline]
    fn serialize_map_key<T: ser::Serialize>(&mut self, state: &mut State, key: T) -> Result<()> {
        self.formatter
            .comma(&mut self.writer, *state == State::First)?;
        *state = State::Rest;

        key.serialize(&mut MapKeySerializer { ser: self })?;

        self.formatter.colon(&mut self.writer)
    }

    #[inline]
    fn serialize_map_value<T: ser::Serialize>(&mut self, _: &mut State, value: T) -> Result<()> {
        value.serialize(self)
    }

    #[inline]
    fn serialize_map_end(&mut self, state: State) -> Result<()> {
        match state {
            State::Empty => Ok(()),
            _ => self.formatter.close(&mut self.writer, b'}'),
        }
    }

    #[inline]
    fn serialize_struct(&mut self, _name: &'static str, len: usize) -> Result<State> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_elt<V: ser::Serialize>(
        &mut self,
        state: &mut State,
        key: &'static str,
        value: V,
    ) -> Result<()> {
        self.serialize_map_key(state, key)?;
        self.serialize_map_value(state, value)
    }

    #[inline]
    fn serialize_struct_end(&mut self, state: State) -> Result<()> {
        self.serialize_map_end(state)
    }

    #[inline]
    fn serialize_struct_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        variant: &'static str,
        len: usize,
    ) -> Result<State> {
        self.formatter.open(&mut self.writer, b'{')?;
        self.formatter.comma(&mut self.writer, true)?;
        escape_key(&mut self.writer, variant)?;
        self.formatter.colon(&mut self.writer)?;
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_variant_elt<V: ser::Serialize>(
        &mut self,
        state: &mut State,
        key: &'static str,
        value: V,
    ) -> Result<()> {
        self.serialize_struct_elt(state, key, value)
    }

    #[inline]
    fn serialize_struct_variant_end(&mut self, state: State) -> Result<()> {
        self.serialize_struct_end(state)?;
        self.formatter.close(&mut self.writer, b'}')
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
    type Error = Error;

    #[inline]
    fn serialize_str(&mut self, value: &str) -> Result<()> {
        escape_key(&mut self.ser.writer, value).map_err(From::from)
    }

    type SeqState = ();
    type TupleState = ();
    type TupleStructState = ();
    type TupleVariantState = ();
    type MapState = ();
    type StructState = ();
    type StructVariantState = ();

    fn serialize_bool(&mut self, _value: bool) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_isize(&mut self, _value: isize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i8(&mut self, _value: i8) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i16(&mut self, _value: i16) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i32(&mut self, _value: i32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_i64(&mut self, _value: i64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_usize(&mut self, _value: usize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u8(&mut self, _value: u8) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u16(&mut self, _value: u16) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u32(&mut self, _value: u32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_u64(&mut self, _value: u64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_f32(&mut self, _value: f32) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_f64(&mut self, _value: f64) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_char(&mut self, _value: char) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_bytes(&mut self, _value: &[u8]) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit(&mut self) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit_struct(&mut self, _name: &'static str) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_unit_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_newtype_struct<T>(&mut self, _name: &'static str, _value: T) -> Result<()>
    where
        T: ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_newtype_variant<T>(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        _variant: &'static str,
        _value: T,
    ) -> Result<()>
    where
        T: ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_none(&mut self) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_some<T>(&mut self, _value: T) -> Result<()>
    where
        T: ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_seq(&mut self, _len: Option<usize>) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_seq_elt<T: ser::Serialize>(&mut self, _state: &mut (), _value: T) -> Result<()>
    where
        T: ser::Serialize,
    {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_seq_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_seq_fixed_size(&mut self, _size: usize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple(&mut self, _len: usize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_elt<T: ser::Serialize>(&mut self, _state: &mut (), _value: T) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_struct(&mut self, _name: &'static str, _len: usize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_struct_elt<T: ser::Serialize>(
        &mut self,
        _state: &mut (),
        _value: T,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_struct_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        _variant: &'static str,
        _len: usize,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_variant_elt<T: ser::Serialize>(
        &mut self,
        _state: &mut (),
        _value: T,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_tuple_variant_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_map(&mut self, _len: Option<usize>) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_map_key<T: ser::Serialize>(&mut self, _state: &mut (), _key: T) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_map_value<T: ser::Serialize>(&mut self, _state: &mut (), _value: T) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_map_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct(&mut self, _name: &'static str, _len: usize) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_elt<V: ser::Serialize>(
        &mut self,
        _state: &mut (),
        _key: &'static str,
        _value: V,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_end(&mut self, _state: ()) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_variant(
        &mut self,
        _name: &'static str,
        _variant_index: usize,
        _variant: &'static str,
        _len: usize,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_variant_elt<V: ser::Serialize>(
        &mut self,
        _state: &mut (),
        _key: &'static str,
        _value: V,
    ) -> Result<()> {
        Err(Error::Syntax(ErrorCode::KeyMustBeAString, 0, 0))
    }

    fn serialize_struct_variant_end(&mut self, _state: ()) -> Result<()> {
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

    fn comma<W>(&mut self, writer: &mut W, _: bool) -> Result<()>
    where
        W: io::Write,
    {
        writer.write_all(b"\n")?;
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
        // NEEDS_ESCAPEML tests if the string can be written as a multiline string (includes needsEscape but without \n, \r, \\ and \")
        static ref NEEDS_ESCAPEML: Regex = Regex::new("'''|[\x00-\x09\x0b\x0c\x0e-\x1f\x7f-\u{9f}\u{00ad}\u{0600}-\u{0604}\u{070f}\u{17b4}\u{17b5}\u{200c}-\u{200f}\u{2028}-\u{202f}\u{2060}-\u{206f}\u{feff}\u{fff0}-\u{ffff}]").expect("Internal error: json parsing");
        // starts with a keyword and optionally is followed by a comment
        static ref STARTS_WITH_KEYWORD: Regex = Regex::new(r#"^(true|false|null)\s*((,|\]|\}|#|//|/\*).*)?$"#).expect("Internal error: json parsing");
    }

    if value.is_empty() {
        formatter.start_value(wr)?;
        return escape_bytes(wr, value.as_bytes());
    }

    // Check if we can insert this string without quotes
    // see hjson syntax (must not parse as true, false, null or number)

    let mut pn = ParseNumber::new(value.bytes());
    let is_number = pn.parse(true).is_ok();

    if is_number || NEEDS_QUOTES.is_match(value) || STARTS_WITH_KEYWORD.is_match(value) {
        // First check if the string can be expressed in multiline format or
        // we must replace the offending characters with safe escape sequences.

        if NEEDS_ESCAPE.is_match(value) && !NEEDS_ESCAPEML.is_match(value)
        /* && !isRootObject */
        {
            ml_str(wr, formatter, value)
        } else {
            formatter.start_value(wr)?;
            escape_bytes(wr, value.as_bytes())
        }
    } else {
        // without quotes
        formatter.start_value(wr)?;
        wr.write_all(value.as_bytes()).map_err(From::from)
    }
}

/// Serializes and escapes a `&str` into a multiline Hjson string.
pub fn ml_str<W, F>(wr: &mut W, formatter: &mut F, value: &str) -> Result<()>
where
    W: io::Write,
    F: Formatter,
{
    // wrap the string into the ''' (multiline) format

    let a: Vec<&str> = value.split('\n').collect();

    if a.len() == 1 {
        // The string contains only a single line. We still use the multiline
        // format as it avoids escaping the \ character (e.g. when used in a
        // regex).
        formatter.start_value(wr)?;
        wr.write_all(b"'''")?;
        wr.write_all(a[0].as_bytes())?;
        wr.write_all(b"'''")?
    } else {
        formatter.newline(wr, 1)?;
        wr.write_all(b"'''")?;
        for line in a {
            formatter.newline(wr, if !line.is_empty() { 1 } else { -999 })?;
            wr.write_all(line.as_bytes())?;
        }
        formatter.newline(wr, 1)?;
        wr.write_all(b"'''")?;
    }
    Ok(())
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

    // Check if we can insert this name without quotes
    if NEEDS_ESCAPE_NAME.is_match(value) {
        escape_bytes(wr, value.as_bytes()).map_err(From::from)
    } else {
        wr.write_all(value.as_bytes()).map_err(From::from)
    }
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
    let f1 = format!("{}", value);
    let f2 = format!("{:e}", value);
    if f1.len() <= f2.len() + 1 {
        f1
    } else if !f2.contains("e-") {
        f2.replace("e", "e+")
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
