//! Wire mapping for stream messages and stream data.

use crate::{StreamData, StreamMessage};
use nu_protocol::{LabeledError, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::value::WireValue;

#[derive(Serialize)]
enum WireStreamData {
    List(WireValue),
    Raw(Result<Vec<u8>, LabeledError>),
}

#[derive(Deserialize)]
enum WireStreamDataDe {
    List(WireValue),
    Raw(Result<Vec<u8>, LabeledError>),
}

impl From<&StreamData> for WireStreamData {
    fn from(data: &StreamData) -> Self {
        match data {
            StreamData::List(value) => Self::List(WireValue::from(value)),
            StreamData::Raw(value) => Self::Raw(value.clone()),
        }
    }
}

impl From<WireStreamDataDe> for StreamData {
    fn from(wire: WireStreamDataDe) -> Self {
        match wire {
            WireStreamDataDe::List(value) => Self::List(Value::from(value)),
            WireStreamDataDe::Raw(value) => Self::Raw(value),
        }
    }
}

#[derive(Serialize)]
enum WireStreamMessage {
    Data(usize, WireStreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Deserialize)]
enum WireStreamMessageDe {
    Data(usize, WireStreamDataDe),
    End(usize),
    Drop(usize),
    Ack(usize),
}

impl From<&StreamMessage> for WireStreamMessage {
    fn from(msg: &StreamMessage) -> Self {
        match msg {
            StreamMessage::Data(id, data) => Self::Data(*id, WireStreamData::from(data)),
            StreamMessage::End(id) => Self::End(*id),
            StreamMessage::Drop(id) => Self::Drop(*id),
            StreamMessage::Ack(id) => Self::Ack(*id),
        }
    }
}

impl From<WireStreamMessageDe> for StreamMessage {
    fn from(wire: WireStreamMessageDe) -> Self {
        match wire {
            WireStreamMessageDe::Data(id, data) => Self::Data(id, data.into()),
            WireStreamMessageDe::End(id) => Self::End(id),
            WireStreamMessageDe::Drop(id) => Self::Drop(id),
            WireStreamMessageDe::Ack(id) => Self::Ack(id),
        }
    }
}

impl Serialize for StreamData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireStreamData::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StreamData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireStreamDataDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for StreamMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireStreamMessage::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StreamMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireStreamMessageDe::deserialize(deserializer)?.into())
    }
}
