//! Wire mapping for evaluated calls and plugin custom values.

use crate::{EvaluatedCall, PluginCustomValue};
use nu_protocol::{Span, Spanned, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::value::WireValue;

#[derive(Serialize)]
struct WireEvaluatedCall {
    head: Span,
    positional: Vec<WireValue>,
    named: Vec<(Spanned<String>, Option<WireValue>)>,
}

#[derive(Deserialize)]
struct WireEvaluatedCallDe {
    head: Span,
    positional: Vec<WireValue>,
    named: Vec<(Spanned<String>, Option<WireValue>)>,
}

impl From<&EvaluatedCall> for WireEvaluatedCall {
    fn from(call: &EvaluatedCall) -> Self {
        Self {
            head: call.head,
            positional: call.positional.iter().map(WireValue::from).collect(),
            named: call
                .named
                .iter()
                .map(|(name, value)| (name.clone(), value.as_ref().map(WireValue::from)))
                .collect(),
        }
    }
}

impl From<WireEvaluatedCallDe> for EvaluatedCall {
    fn from(wire: WireEvaluatedCallDe) -> Self {
        Self {
            head: wire.head,
            positional: wire.positional.into_iter().map(Value::from).collect(),
            named: wire
                .named
                .into_iter()
                .map(|(name, value)| (name, value.map(Value::from)))
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct WirePluginCustomValue<'a> {
    name: &'a str,
    data: &'a [u8],
    #[serde(default, skip_serializing_if = "is_false")]
    notify_on_drop: bool,
}

#[derive(Deserialize)]
struct WirePluginCustomValueDe {
    name: String,
    data: Vec<u8>,
    #[serde(default)]
    notify_on_drop: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl Serialize for EvaluatedCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireEvaluatedCall::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EvaluatedCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireEvaluatedCallDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for PluginCustomValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginCustomValue {
            name: self.name(),
            data: self.data(),
            notify_on_drop: self.notify_on_drop(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PluginCustomValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WirePluginCustomValueDe::deserialize(deserializer)?;
        Ok(Self::new(wire.name, wire.data, wire.notify_on_drop))
    }
}
