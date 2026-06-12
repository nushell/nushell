use std::{cell::RefCell, fmt::Write};

use crate::yaml::Spec;
use chrono::{DateTime, FixedOffset};
use derive_setters::Setters;
use nu_protocol::{
    Range, ShellError, Span, Value, ast::CellPath, engine::Closure,
    shell_error::generic::GenericError,
};
use nu_utils::FmtHandle;
use serde::{Serialize, ser::SerializeMap};
use serde_saphyr::{FlowMap, FlowSeq, FoldStr, LitStr, Serializer, ser_options};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct SerializeOptions {
    spec: Spec,
}

pub fn serialize(
    value: &Value,
    span: Span,
    options: &SerializeOptions,
) -> Result<String, ShellError> {
    let mut options = ser_options! {
        yaml_12: match options.spec {
            Spec::V1_1 => false,
            Spec::V1_2 => true,
        },
    };

    let value = YamlValue::try_from_value(value, span)?;
    let mut writer = FmtHandle::new(String::new());
    WRITER.set(Some(writer.clone()));
    let mut serializer = Serializer::with_options(&mut writer, &mut options);

    // Clear out any preambles by the serializer
    ().serialize(&mut serializer).unwrap();
    WRITER.with_borrow_mut(|writer| {
        if let Some(writer) = writer {
            writer.take();
        }
    });

    value.serialize(&mut serializer).unwrap();
    Ok(writer.take())
}

thread_local! {
    static WRITER: RefCell<Option<FmtHandle<String>>> = RefCell::new(None);
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
        let tag = self.tag();
        fn serialize_with_tag<S: serde::Serializer>(
            serializer: S,
            tag: &str,
            value: impl Serialize,
        ) -> Result<S::Ok, S::Error> {
            WRITER.with_borrow_mut(|writer| {
                let writer = writer
                    .as_mut()
                    .expect("writer set before calling any serialization");
                writer.write_char(' ').map_err(|_| todo!())?;
                writer.write_str(tag).map_err(|_| todo!())?;
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
            YamlValue::Closure(_closure) => todo!(),
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
        for (key, value) in self.0.iter() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'v> YamlValue<'v> {
    fn tag(&self) -> &'static str {
        match self {
            YamlValue::Bool(_) => "!!bool",
            YamlValue::Int(_) => "!!int",
            YamlValue::Float(_) => "!!float",
            YamlValue::Str(_) | YamlValue::FoldStr(_) | YamlValue::LitStr(_) => "!!str",
            YamlValue::Map(_) | YamlValue::FlowMap(_) => "!!map",
            YamlValue::Seq(_) | YamlValue::FlowSeq(_) => "!!seq",
            YamlValue::Null => "!!null",

            YamlValue::Glob(_) => "!glob",
            YamlValue::Filesize(_) => "!filesize",
            YamlValue::Duration(_) => "!duration",
            YamlValue::Date(_) => "!date",
            YamlValue::Range(_) => "!range",
            YamlValue::Closure(_) => "!closure",
            YamlValue::Error(_) => "!error",
            YamlValue::Binary(_) => "!!binary",
            YamlValue::CellPath(_) => "!cell-path",
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
