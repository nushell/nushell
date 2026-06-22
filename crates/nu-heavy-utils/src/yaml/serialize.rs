use crate::yaml::{KnownTag, Spec};
use chrono::{DateTime, FixedOffset};
use derive_setters::Setters;
use nu_protocol::{
    Range, ShellError, Span, Value,
    ast::CellPath,
    engine::{Closure, EngineState},
    shell_error::generic::GenericError,
};
use nu_utils::FmtHandle;
use scopeguard::defer;
use serde::{Serialize, ser::SerializeMap};
use serde_saphyr::{FlowMap, FlowSeq, FoldStr, LitStr, Serializer, ser_options};
use std::{
    cell::{Cell, RefCell},
    fmt::{self, Write},
    io,
};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct SerializeOptions {
    spec: Spec,

    /// Controls how values are serialized when they cannot be represented in a
    /// way that can be deserialized back into the original type.
    non_roundtrip: NonRoundtrip,

    /// Expect a list of values and then construct a multi document YAML output.
    multiple: bool,

    /// Add directives to the start of the document that explains YAML version and the nushell tag.
    add_directives: bool,
}

/// Controls how non-round-trippable values are serialized.
///
/// Some values can be serialized, but cannot be deserialized back into their
/// original type without losing information. This option decides whether such
/// values are replaced with `null` or emitted in a lossy representation.
#[derive(Debug, Clone, Default)]
pub enum NonRoundtrip {
    /// Serialize non-round-trippable values as `null`.
    #[default]
    Null,

    /// Serialize non-round-trippable values using a lossy representation.
    Lossy {
        /// Engine state is required to serialize closures this way.
        engine_state: EngineState,
    },
}

pub fn serialize(
    value: &Value,
    span: Span,
    options: SerializeOptions,
) -> Result<String, ShellError> {
    let spec = options.spec;
    let multiple = options.multiple;
    let add_directives = options.add_directives;

    let mut ser_options = ser_options! {
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
    ().serialize(&mut serializer).unwrap();
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
            value.serialize(&mut serializer).unwrap();

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
        value.serialize(&mut serializer).unwrap();
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
                    writer.write_char(' ').map_err(|_| todo!())?;
                }

                writer.write_str(tag).map_err(|_| todo!())?;

                if !IN_MAP.get() {
                    writer.write_char(' ').map_err(|_| todo!())?;
                }

                value.serialize(serializer)
            })
        }

        match self {
            // untagged types
            YamlValue::Bool(bool) => bool.serialize(serializer),
            YamlValue::Int(int) => int.serialize(serializer),
            YamlValue::Float(float) => float.serialize(serializer),
            YamlValue::Str(str) => str.serialize(serializer),
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
            YamlValue::Closure(closure) => {
                OPTIONS.with_borrow(|options| match &options.non_roundtrip {
                    NonRoundtrip::Null => serializer.serialize_unit(),
                    NonRoundtrip::Lossy { engine_state } => {
                        let block = engine_state.get_block(closure.block_id);
                        if let Some(span) = block.span {
                            let contents = engine_state.get_span_contents(span);
                            let contents = String::from_utf8_lossy(contents);
                            serialize_with_tag(serializer, tag, contents)
                        } else {
                            todo!("throw error that content could not be found")
                        }
                    }
                })
            }
            YamlValue::Error(shell_error) => serialize_with_tag(serializer, tag, shell_error),
            YamlValue::Binary(items) => serializer.serialize_bytes(items),
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
            YamlValue::Date(_) => KnownTag::Date,
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
            Value::Custom { .. } => {
                // TODO: implement structure style values here
                return Err(ShellError::Generic(
                    GenericError::new(
                        "Unsupported custom values",
                        "Cannot convert custom values into YAML",
                        span,
                    )
                    .with_code("shell::yaml::serialize::unsupported_custom_value")
                    .with_help("Try to call `into value` on the custom value first"),
                ));
            }
            Value::Nothing { .. } => YamlValue::Null,
        })
    }
}
