//! YAML serialization.
//!
//! This module implements the YAML serializer by using [`serde_saphyr`].
//!
//! Usually, [`serde_saphyr`] is designed without an intermediate YAML value representation.
//! It serializes application data types directly instead.
//!
//! For us, [`Value`] is already the data model we need to preserve pretty much as is.
//! To get a more controlled conversion into YAML, we use [`YamlValue`] here.
//! This lets us bypass the [`Serialize`] and [`Deserialize`](serde::Deserialize)
//! implementations directly on [`Value`] and use our own more specific handling.
//!
//! The serializer is built around [`serde`], but that limits how much extra context we can pass
//! into serialization.
//! To work around this, we use [`thread_local`]s to keep context data around even though we cannot
//! pass it through the serializer directly.
//!
//! Since serialization never leaves the current thread, this is safe while still allowing multiple
//! threads to serialize to YAML at the same time.
//! This design is pretty delicate though, so nothing from the serializer internals is exposed.
//!
//! For now, the serializer does not support tags directly.
//! To handle this, we use [`FmtHandle`] to access the underlying string directly and write to it
//! around the [`YamlSerializer`](serde_saphyr::ser::YamlSerializer).
//!
//! This documentation is private to the implementors, as this module itself is not public.
//! Only [`serialize`], [`SerializeOptions`], and their field types are public.

use crate::yaml::{KnownTag, Spec, error::SerializeError};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use chrono::{DateTime, FixedOffset};
use derive_setters::Setters;
use nu_protocol::{
    FromValue, Range, ShellError, Span, Value,
    ast::CellPath,
    engine::{Closure, EngineState},
};
use nu_utils::FmtHandle;
use scopeguard::defer;
use serde::{
    Serialize,
    ser::{Error, SerializeMap},
};
use serde_saphyr::{
    DoubleQuoted, FlowMap, FlowSeq, FoldStr, LitStr, Serializer, SingleQuoted, ser_options,
};
use std::{
    cell::{Cell, RefCell},
    fmt::{self, Write},
};

/// Options for serializing YAML.
///
/// Use this to configure how the serializer works.
///
/// This type provides builder-style setters directly, so options can be chained while building it.
///
/// ```rust
/// # use nu_heavy_utils::yaml::*;
/// #
/// let options = SerializeOptions::default()
///     .with_spec(Spec::V1_2)
///     .with_multiple(true)
///     .with_add_directives(true)
///     .with_indent(4);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Setters, better_default::Default)]
#[setters(prefix = "with_")]
pub struct SerializeOptions {
    /// Configure which YAML spec to follow.
    pub spec: Spec,

    /// Configure how values are serialized when they cannot round-trip cleanly.
    ///
    /// Some values can be written to YAML, but cannot be parsed back into the original type without
    /// losing information.
    /// This option controls how those values are handled.
    pub non_roundtrip: NonRoundtrip,

    /// Treat the input as a list of documents and serialize it as a multi-document YAML stream.
    pub multiple: bool,

    /// Add directives at the start of the document.
    ///
    /// These include the YAML version and the Nushell tag directive.
    pub add_directives: bool,

    /// Configure how many spaces are used for indentation.
    #[default(2)]
    pub indent: usize,

    /// Use compact indentation for nested lists.
    #[default(true)]
    pub compact_list_indent: bool,

    /// Configure how strings are quoted.
    pub quote_style: QuoteStyle,
}

/// Configure how non-round-trippable values are serialized.
#[derive(Debug, Clone, Default)]
pub enum NonRoundtrip {
    /// Serialize non-round-trippable values as `null`.
    #[default]
    Null,

    /// Serialize non-round-trippable values using a lossy representation.
    ///
    /// This keeps more information in the YAML output, but it might not deserialize back into the
    /// exact original type.
    Lossy {
        /// Engine state is required to serialize closures this way.
        engine_state: EngineState,
    },
}

/// Configure how strings are quoted.
#[derive(Debug, Clone, Copy, Default, FromValue)]
pub enum QuoteStyle {
    /// Pick the quote style automatically.
    #[default]
    Auto,

    /// Use single quotes for strings.
    Single,

    /// Use double quotes for strings.
    Double,
}

/// Serialize a [`Value`] into a YAML string.
///
/// See [`SerializeOptions`] for configurable output behavior.
/// `span` is used for serialization errors.
pub fn serialize(
    value: &Value,
    span: Span,
    options: SerializeOptions,
) -> Result<String, ShellError> {
    let spec = options.spec;
    let multiple = options.multiple;
    let add_directives = options.add_directives;

    let mut ser_options = ser_options! {
        indent_step: options.indent,
        compact_list_indent: options.compact_list_indent,
        yaml_12: match spec {
            Spec::V1_1 => false,
            Spec::V1_2 => true,
        },
    };

    // set options to thread local but remove it after to free the potential lingering engine state
    OPTIONS.set(options);
    defer!(OPTIONS.set(SerializeOptions::default()));

    let mut writer = FmtHandle::new(String::new());
    WRITER.set(Some(writer.clone()));
    IN_MAP.set(false);
    let mut serializer = Serializer::with_options(&mut writer, &mut ser_options);

    // attention: suppresses any output from f, also don't call serializer inside this
    fn with_writer<F: Fn(&mut FmtHandle<String>) -> R, R>(f: F) {
        WRITER.with_borrow_mut(|writer| {
            if let Some(writer) = writer {
                f(writer);
            }
        })
    }

    // clear out any preambles by the serializer
    ().serialize(&mut serializer)
        .map_err(|err| SerializeError::Serializer { err, span })?;
    with_writer(|writer| writer.take());

    let write_directives = |writer: &mut FmtHandle<String>| {
        match spec {
            Spec::V1_1 => writeln!(writer, "%YAML 1.1"),
            Spec::V1_2 => writeln!(writer, "%YAML 1.2"),
        }?;
        writeln!(writer, "%TAG ! {}", KnownTag::NUSHELL_PREFIX)?;
        fmt::Result::Ok(())
    };

    if multiple {
        let values = value.as_list()?;
        let mut first = true;
        for value in values {
            if add_directives {
                with_writer(|writer| {
                    write_directives(writer)?;
                    writeln!(writer, "---")?;
                    fmt::Result::Ok(())
                });
            } else {
                match first {
                    true => first = false,
                    false => with_writer(|writer| writeln!(writer, "---")),
                }
            }

            let value = YamlValue::try_from_value(value, span)?;
            value
                .serialize(&mut serializer)
                .map_err(|err| SerializeError::Serializer { err, span })?;

            if add_directives {
                with_writer(|writer| writeln!(writer, "..."));
            }
        }
    } else {
        if add_directives {
            with_writer(|writer| {
                write_directives(writer)?;
                writeln!(writer, "---")?;
                fmt::Result::Ok(())
            });
        }

        let value = YamlValue::try_from_value(value, span)?;
        value
            .serialize(&mut serializer)
            .map_err(|err| SerializeError::Serializer { err, span })?;
    }

    Ok(writer.take())
}

thread_local! {
    static WRITER: RefCell<Option<FmtHandle<String>>> = RefCell::new(None);
    static IN_MAP: Cell<bool> = Cell::new(false);
    static OPTIONS: RefCell<SerializeOptions> = RefCell::new(SerializeOptions::default());
}

#[expect(
    unused,
    reason = "in the future we may store styles of values, then these would allow restoring them"
)]
enum YamlValue<'v> {
    // untagged types
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'v str),
    FoldStr(FoldStr<'v>),
    LitStr(LitStr<'v>),
    Map(YamlMap<'v>),
    FlowMap(FlowMap<Vec<(&'v str, YamlValue<'v>)>>),
    Seq(Vec<YamlValue<'v>>),
    FlowSeq(FlowSeq<Vec<YamlValue<'v>>>),
    Null,

    // tagged types
    Glob(&'v str),
    Filesize(i64),
    Duration(i64),
    Date(&'v DateTime<FixedOffset>),
    Range(&'v Range),
    Closure(&'v Closure),
    Error(&'v ShellError),
    Binary(&'v [u8]),
    CellPath(&'v CellPath),
}

impl Serialize for YamlValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let tag = self.tag().to_string();
        let tag = tag.as_str();
        fn serialize_with_tag<S: serde::Serializer>(
            serializer: S,
            tag: &str,
            value: impl Serialize,
        ) -> Result<S::Ok, S::Error> {
            WRITER.with_borrow_mut(|writer| {
                let writer = writer
                    .as_mut()
                    .expect("writer set before calling any serialization");

                if IN_MAP.get() {
                    writer
                        .write_char(' ')
                        .expect("infallible for writes to string");
                }

                writer
                    .write_str(tag)
                    .expect("infallible for writes to string");

                if !IN_MAP.get() {
                    writer
                        .write_char(' ')
                        .expect("infallible for writes to string");
                }

                value.serialize(serializer)
            })
        }

        let serialize_closure = |closure: &Closure, serializer: S| {
            OPTIONS.with_borrow(|options| match &options.non_roundtrip {
                NonRoundtrip::Null => serializer.serialize_unit(),
                NonRoundtrip::Lossy { engine_state } => {
                    let block = engine_state.get_block(closure.block_id);
                    if let Some(span) = block.span {
                        let contents = engine_state.get_span_contents(span);
                        let contents = String::from_utf8_lossy(contents);
                        serialize_with_tag(serializer, tag, contents)
                    } else {
                        Err(S::Error::custom(SerializeError::CLOSURE_SPAN_NOT_FOUND))
                    }
                }
            })
        };

        let serialize_str = |str: &str, serializer: S| {
            OPTIONS.with_borrow(|options| match options.quote_style {
                QuoteStyle::Auto => str.serialize(serializer),
                QuoteStyle::Single => SingleQuoted(str).serialize(serializer),
                QuoteStyle::Double => DoubleQuoted(str).serialize(serializer),
            })
        };

        let serialize_error = |error: &ShellError, serializer: S| {
            OPTIONS.with_borrow(|options| match &options.non_roundtrip {
                NonRoundtrip::Null => serializer.serialize_unit(),
                NonRoundtrip::Lossy { .. } => serialize_with_tag(serializer, tag, error),
            })
        };

        let serialize_binary = |bytes: &[u8], serializer: S| {
            serialize_with_tag(serializer, tag, BASE64_STANDARD.encode(bytes))
        };

        match self {
            // untagged types
            YamlValue::Bool(bool) => bool.serialize(serializer),
            YamlValue::Int(int) => int.serialize(serializer),
            YamlValue::Float(float) => float.serialize(serializer),
            YamlValue::Str(str) => serialize_str(str, serializer),
            YamlValue::FoldStr(fold_str) => fold_str.serialize(serializer),
            YamlValue::LitStr(lit_str) => lit_str.serialize(serializer),
            YamlValue::Map(yaml_map) => yaml_map.serialize(serializer),
            YamlValue::FlowMap(flow_map) => flow_map.serialize(serializer),
            YamlValue::Seq(yaml_values) => yaml_values.serialize(serializer),
            YamlValue::FlowSeq(flow_seq) => flow_seq.serialize(serializer),
            YamlValue::Null => serializer.serialize_unit(),

            // tagged types, requires a bit more work
            YamlValue::Glob(glob) => serialize_with_tag(serializer, tag, glob),
            YamlValue::Filesize(filesize) => serialize_with_tag(serializer, tag, filesize),
            YamlValue::Duration(duration) => serialize_with_tag(serializer, tag, duration),
            YamlValue::Date(date_time) => serialize_with_tag(serializer, tag, date_time),
            YamlValue::Range(range) => serialize_with_tag(serializer, tag, range),
            YamlValue::Closure(closure) => serialize_closure(closure, serializer),
            YamlValue::Error(shell_error) => serialize_error(shell_error, serializer),
            YamlValue::Binary(bytes) => serialize_binary(bytes, serializer),
            YamlValue::CellPath(cell_path) => serialize_with_tag(serializer, tag, cell_path),
        }
    }
}

struct YamlMap<'v>(Vec<(&'v str, YamlValue<'v>)>);

impl Serialize for YamlMap<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(self.0.len().into())?;
        IN_MAP.set(true);
        for (key, value) in self.0.iter() {
            map.serialize_entry(key, value)?;
        }
        IN_MAP.set(false);
        map.end()
    }
}

impl<'v> YamlValue<'v> {
    fn tag(&self) -> KnownTag {
        match self {
            YamlValue::Bool(_) => KnownTag::Bool,
            YamlValue::Int(_) => KnownTag::Int,
            YamlValue::Float(_) => KnownTag::Float,
            YamlValue::Str(_) | YamlValue::FoldStr(_) | YamlValue::LitStr(_) => KnownTag::Str,
            YamlValue::Map(_) | YamlValue::FlowMap(_) => KnownTag::Map,
            YamlValue::Seq(_) | YamlValue::FlowSeq(_) => KnownTag::Seq,
            YamlValue::Null => KnownTag::Null,

            YamlValue::Glob(_) => KnownTag::Glob,
            YamlValue::Filesize(_) => KnownTag::Filesize,
            YamlValue::Duration(_) => KnownTag::Duration,
            YamlValue::Date(_) => KnownTag::Timestamp,
            YamlValue::Range(_) => KnownTag::Range,
            YamlValue::Closure(_) => KnownTag::Closure,
            YamlValue::Error(_) => KnownTag::Error,
            YamlValue::Binary(_) => KnownTag::Binary,
            YamlValue::CellPath(_) => KnownTag::CellPath,
        }
    }

    fn try_from_value(value: &'v Value, span: Span) -> Result<Self, ShellError> {
        Ok(match value {
            Value::Bool { val, .. } => YamlValue::Bool(*val),
            Value::Int { val, .. } => YamlValue::Int(*val),
            Value::Float { val, .. } => YamlValue::Float(*val),
            Value::String { val, .. } => YamlValue::Str(val.as_str()),
            Value::Glob { val, .. } => YamlValue::Glob(val.as_str()),
            Value::Filesize { val, .. } => YamlValue::Filesize(val.get()),
            Value::Duration { val, .. } => YamlValue::Duration(*val),
            Value::Date { val, .. } => YamlValue::Date(val),
            Value::Range { val, .. } => YamlValue::Range(&*val),
            Value::Record { val, .. } => {
                let mut values = Vec::with_capacity(val.len());
                for (k, v) in val.iter() {
                    let v = YamlValue::try_from_value(v, span)?;
                    values.push((k.as_str(), v));
                }
                YamlValue::Map(YamlMap(values))
            }
            Value::List { vals, .. } => {
                let mut values = Vec::with_capacity(vals.len());
                for val in vals.iter() {
                    let val = YamlValue::try_from_value(val, span)?;
                    values.push(val);
                }
                YamlValue::Seq(values)
            }
            Value::Closure { val, .. } => YamlValue::Closure(&*val),
            Value::Error { error, .. } => YamlValue::Error(&*error),
            Value::Binary { val, .. } => YamlValue::Binary(val.as_slice()),
            Value::CellPath { val, .. } => YamlValue::CellPath(val),
            Value::Custom { val, .. } => {
                // TODO: implement structure style values here

                return Err(ShellError::from(SerializeError::UnsupportedCustomValue {
                    type_name: val.type_name(),
                    span,
                }));
            }
            Value::Nothing { .. } => YamlValue::Null,
        })
    }
}
