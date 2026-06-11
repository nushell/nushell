use crate::yaml::Spec;
use derive_setters::Setters;
use nu_protocol::{ShellError, Span, Value};
use serde::{Serialize, ser::SerializeMap};
use serde_saphyr::{FlowMap, FlowSeq, FoldStr, LitStr};

#[non_exhaustive]
#[derive(Debug, Clone, Default, Setters)]
pub struct SerializeOptions {
    spec: Spec,
}

pub fn serialize(value: &Value, span: Span, options: &SerializeOptions) -> Result<String, ShellError> {
    let value = YamlValue::from(value);
    serde_saphyr::to_string(&value).map_err(|_err| todo!())
}

#[derive(Serialize)]
#[serde(untagged)]
enum YamlValue<'v> {
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
}

struct YamlMap<'v>(Vec<(&'v str, YamlValue<'v>)>);

impl Serialize for YamlMap<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut map = serializer.serialize_map(self.0.len().into())?;
        for (key, value) in self.0.iter() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'v> From<&'v Value> for YamlValue<'v> {
    fn from(value: &'v Value) -> Self {
        match value {
            Value::Bool { val, .. } => YamlValue::Bool(*val),
            Value::Int { val, .. } => YamlValue::Int(*val),
            Value::Float { val, .. } => YamlValue::Float(*val),
            Value::String { val, .. } => YamlValue::Str(val.as_str()),
            Value::Glob { val, .. } => todo!(),
            Value::Filesize { val, .. } => todo!(),
            Value::Duration { val, .. } => todo!(),
            Value::Date { val, .. } => todo!(),
            Value::Range { val, .. } => todo!(),
            Value::Record { val, .. } => YamlValue::Map(YamlMap(Vec::from_iter(val.iter().map(|(k, v)| (k.as_str(), v.into()))))),
            Value::List { vals, .. } => YamlValue::Seq(Vec::from_iter(vals.iter().map(|v| v.into()))),
            Value::Closure { val, .. } => todo!(),
            Value::Error { error, .. } => todo!(),
            Value::Binary { val, .. } => todo!(),
            Value::CellPath { val, .. } => todo!(),
            Value::Custom { val, .. } => todo!(),
            Value::Nothing { .. } => YamlValue::Null,
        }
    }
}
