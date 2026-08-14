//! Wire mapping for plugin calls, custom value ops, and call responses.

use crate::{
    CallInfo, CustomValueOp, DynamicCompletionCall, EvaluatedCall, GetCompletionArgType,
    GetCompletionInfo, Ordering, PluginCall, PluginCallResponse, PluginCustomValue,
};
use nu_protocol::{ShellError, Spanned, Value, ast::Operator, casing::Casing};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;

use super::value::WireValue;

#[derive(Serialize)]
struct WireCallInfo<'a, D: Serialize> {
    name: &'a str,
    call: &'a EvaluatedCall,
    input: &'a D,
}

#[derive(Deserialize)]
struct WireCallInfoDe<D> {
    name: String,
    call: EvaluatedCall,
    input: D,
}

#[derive(Serialize)]
enum WireGetCompletionArgType<'a> {
    Flag(&'a str),
    Positional(usize),
}

#[derive(Deserialize)]
enum WireGetCompletionArgTypeDe {
    Flag(String),
    Positional(usize),
}

impl<'a> From<&'a GetCompletionArgType> for WireGetCompletionArgType<'a> {
    fn from(arg: &'a GetCompletionArgType) -> Self {
        match arg {
            GetCompletionArgType::Flag(name) => Self::Flag(name),
            GetCompletionArgType::Positional(index) => Self::Positional(*index),
        }
    }
}

impl From<WireGetCompletionArgTypeDe> for GetCompletionArgType {
    fn from(wire: WireGetCompletionArgTypeDe) -> Self {
        match wire {
            WireGetCompletionArgTypeDe::Flag(name) => Self::Flag(name),
            WireGetCompletionArgTypeDe::Positional(index) => Self::Positional(index),
        }
    }
}

#[derive(Serialize)]
struct WireDynamicCompletionCall<'a> {
    call: &'a nu_protocol::ast::Call,
    strip: bool,
    pos: usize,
}

#[derive(Deserialize)]
struct WireDynamicCompletionCallDe {
    call: nu_protocol::ast::Call,
    strip: bool,
    pos: usize,
}

impl<'a> From<&'a DynamicCompletionCall> for WireDynamicCompletionCall<'a> {
    fn from(call: &'a DynamicCompletionCall) -> Self {
        Self {
            call: &call.call,
            strip: call.strip,
            pos: call.pos,
        }
    }
}

impl From<WireDynamicCompletionCallDe> for DynamicCompletionCall {
    fn from(wire: WireDynamicCompletionCallDe) -> Self {
        Self {
            call: wire.call,
            strip: wire.strip,
            pos: wire.pos,
        }
    }
}

#[derive(Serialize)]
struct WireGetCompletionInfo<'a> {
    name: &'a str,
    arg_type: WireGetCompletionArgType<'a>,
    call: WireDynamicCompletionCall<'a>,
}

#[derive(Deserialize)]
struct WireGetCompletionInfoDe {
    name: String,
    arg_type: WireGetCompletionArgTypeDe,
    call: WireDynamicCompletionCallDe,
}

impl<'a> From<&'a GetCompletionInfo> for WireGetCompletionInfo<'a> {
    fn from(info: &'a GetCompletionInfo) -> Self {
        Self {
            name: &info.name,
            arg_type: WireGetCompletionArgType::from(&info.arg_type),
            call: WireDynamicCompletionCall::from(&info.call),
        }
    }
}

impl From<WireGetCompletionInfoDe> for GetCompletionInfo {
    fn from(wire: WireGetCompletionInfoDe) -> Self {
        Self {
            name: wire.name,
            arg_type: wire.arg_type.into(),
            call: wire.call.into(),
        }
    }
}

#[derive(Serialize)]
enum WirePluginCall<'a, D: Serialize> {
    Metadata,
    Signature,
    Run(WireCallInfo<'a, D>),
    GetCompletion(WireGetCompletionInfo<'a>),
    CustomValueOp(&'a Spanned<PluginCustomValue>, WireCustomValueOp),
}

#[derive(Deserialize)]
enum WirePluginCallDe<D> {
    Metadata,
    Signature,
    Run(WireCallInfoDe<D>),
    GetCompletion(WireGetCompletionInfoDe),
    CustomValueOp(Spanned<PluginCustomValue>, WireCustomValueOpDe),
}

impl<'a, D: Serialize> From<&'a PluginCall<D>> for WirePluginCall<'a, D> {
    fn from(call: &'a PluginCall<D>) -> Self {
        match call {
            PluginCall::Metadata => Self::Metadata,
            PluginCall::Signature => Self::Signature,
            PluginCall::Run(info) => Self::Run(WireCallInfo {
                name: &info.name,
                call: &info.call,
                input: &info.input,
            }),
            PluginCall::GetCompletion(info) => {
                Self::GetCompletion(WireGetCompletionInfo::from(info))
            }
            PluginCall::CustomValueOp(value, op) => {
                Self::CustomValueOp(value, WireCustomValueOp::from(op))
            }
        }
    }
}

impl<D> From<WirePluginCallDe<D>> for PluginCall<D> {
    fn from(wire: WirePluginCallDe<D>) -> Self {
        match wire {
            WirePluginCallDe::Metadata => Self::Metadata,
            WirePluginCallDe::Signature => Self::Signature,
            WirePluginCallDe::Run(info) => Self::Run(CallInfo {
                name: info.name,
                call: info.call,
                input: info.input,
            }),
            WirePluginCallDe::GetCompletion(info) => Self::GetCompletion(info.into()),
            WirePluginCallDe::CustomValueOp(value, op) => Self::CustomValueOp(value, op.into()),
        }
    }
}

#[derive(Serialize)]
enum WireCustomValueOp {
    ToBaseValue,
    FollowPathInt {
        index: Spanned<usize>,
        optional: bool,
    },
    FollowPathString {
        column_name: Spanned<String>,
        optional: bool,
        casing: Casing,
    },
    PartialCmp(WireValue),
    Operation(Spanned<Operator>, WireValue),
    Save {
        path: Spanned<PathBuf>,
        save_call_span: nu_protocol::Span,
    },
    Dropped,
}

#[derive(Deserialize)]
enum WireCustomValueOpDe {
    ToBaseValue,
    FollowPathInt {
        index: Spanned<usize>,
        optional: bool,
    },
    FollowPathString {
        column_name: Spanned<String>,
        optional: bool,
        casing: Casing,
    },
    PartialCmp(WireValue),
    Operation(Spanned<Operator>, WireValue),
    Save {
        path: Spanned<PathBuf>,
        save_call_span: nu_protocol::Span,
    },
    Dropped,
}

impl From<&CustomValueOp> for WireCustomValueOp {
    fn from(op: &CustomValueOp) -> Self {
        match op {
            CustomValueOp::ToBaseValue => Self::ToBaseValue,
            CustomValueOp::FollowPathInt { index, optional } => Self::FollowPathInt {
                index: *index,
                optional: *optional,
            },
            CustomValueOp::FollowPathString {
                column_name,
                optional,
                casing,
            } => Self::FollowPathString {
                column_name: column_name.clone(),
                optional: *optional,
                casing: *casing,
            },
            CustomValueOp::PartialCmp(value) => Self::PartialCmp(WireValue::from(value)),
            CustomValueOp::Operation(operator, value) => {
                Self::Operation(*operator, WireValue::from(value))
            }
            CustomValueOp::Save {
                path,
                save_call_span,
            } => Self::Save {
                path: path.clone(),
                save_call_span: *save_call_span,
            },
            CustomValueOp::Dropped => Self::Dropped,
        }
    }
}

impl From<WireCustomValueOpDe> for CustomValueOp {
    fn from(wire: WireCustomValueOpDe) -> Self {
        match wire {
            WireCustomValueOpDe::ToBaseValue => Self::ToBaseValue,
            WireCustomValueOpDe::FollowPathInt { index, optional } => {
                Self::FollowPathInt { index, optional }
            }
            WireCustomValueOpDe::FollowPathString {
                column_name,
                optional,
                casing,
            } => Self::FollowPathString {
                column_name,
                optional,
                casing,
            },
            WireCustomValueOpDe::PartialCmp(value) => Self::PartialCmp(Value::from(value)),
            WireCustomValueOpDe::Operation(operator, value) => {
                Self::Operation(operator, Value::from(value))
            }
            WireCustomValueOpDe::Save {
                path,
                save_call_span,
            } => Self::Save {
                path,
                save_call_span,
            },
            WireCustomValueOpDe::Dropped => Self::Dropped,
        }
    }
}

#[derive(Serialize)]
enum WirePluginCallResponse<'a, D: Serialize> {
    Ok,
    Error(&'a ShellError),
    Metadata(&'a nu_protocol::PluginMetadata),
    Signature(&'a Vec<nu_protocol::PluginSignature>),
    Ordering(Option<WireOrdering>),
    CompletionItems(&'a Option<Vec<nu_protocol::DynamicSuggestion>>),
    PipelineData(&'a D),
}

#[derive(Deserialize)]
enum WirePluginCallResponseDe<D> {
    Ok,
    Error(ShellError),
    Metadata(nu_protocol::PluginMetadata),
    Signature(Vec<nu_protocol::PluginSignature>),
    Ordering(Option<WireOrdering>),
    CompletionItems(Option<Vec<nu_protocol::DynamicSuggestion>>),
    PipelineData(D),
}

impl<'a, D: Serialize> From<&'a PluginCallResponse<D>> for WirePluginCallResponse<'a, D> {
    fn from(response: &'a PluginCallResponse<D>) -> Self {
        match response {
            PluginCallResponse::Ok => Self::Ok,
            PluginCallResponse::Error(err) => Self::Error(err),
            PluginCallResponse::Metadata(meta) => Self::Metadata(meta),
            PluginCallResponse::Signature(sigs) => Self::Signature(sigs),
            PluginCallResponse::Ordering(ordering) => {
                Self::Ordering(ordering.map(WireOrdering::from))
            }
            PluginCallResponse::CompletionItems(items) => Self::CompletionItems(items),
            PluginCallResponse::PipelineData(data) => Self::PipelineData(data),
        }
    }
}

impl<D> From<WirePluginCallResponseDe<D>> for PluginCallResponse<D> {
    fn from(wire: WirePluginCallResponseDe<D>) -> Self {
        match wire {
            WirePluginCallResponseDe::Ok => Self::Ok,
            WirePluginCallResponseDe::Error(err) => Self::Error(err),
            WirePluginCallResponseDe::Metadata(meta) => Self::Metadata(meta),
            WirePluginCallResponseDe::Signature(sigs) => Self::Signature(sigs),
            WirePluginCallResponseDe::Ordering(ordering) => {
                Self::Ordering(ordering.map(Ordering::from))
            }
            WirePluginCallResponseDe::CompletionItems(items) => Self::CompletionItems(items),
            WirePluginCallResponseDe::PipelineData(data) => Self::PipelineData(data),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum WireOrdering {
    Less,
    Equal,
    Greater,
}

impl From<Ordering> for WireOrdering {
    fn from(ordering: Ordering) -> Self {
        match ordering {
            Ordering::Less => Self::Less,
            Ordering::Equal => Self::Equal,
            Ordering::Greater => Self::Greater,
        }
    }
}

impl From<WireOrdering> for Ordering {
    fn from(wire: WireOrdering) -> Self {
        match wire {
            WireOrdering::Less => Self::Less,
            WireOrdering::Equal => Self::Equal,
            WireOrdering::Greater => Self::Greater,
        }
    }
}

impl<D> Serialize for CallInfo<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireCallInfo {
            name: &self.name,
            call: &self.call,
            input: &self.input,
        }
        .serialize(serializer)
    }
}

impl<'de, D> Deserialize<'de> for CallInfo<D>
where
    D: Deserialize<'de>,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: Deserializer<'de>,
    {
        let wire = WireCallInfoDe::deserialize(deserializer)?;
        Ok(Self {
            name: wire.name,
            call: wire.call,
            input: wire.input,
        })
    }
}

impl Serialize for GetCompletionArgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireGetCompletionArgType::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GetCompletionArgType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireGetCompletionArgTypeDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for DynamicCompletionCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireDynamicCompletionCall::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DynamicCompletionCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireDynamicCompletionCallDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for GetCompletionInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireGetCompletionInfo::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GetCompletionInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireGetCompletionInfoDe::deserialize(deserializer)?.into())
    }
}

impl<D> Serialize for PluginCall<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginCall::from(self).serialize(serializer)
    }
}

impl<'de, D> Deserialize<'de> for PluginCall<D>
where
    D: Deserialize<'de>,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: Deserializer<'de>,
    {
        Ok(WirePluginCallDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for CustomValueOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireCustomValueOp::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CustomValueOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireCustomValueOpDe::deserialize(deserializer)?.into())
    }
}

impl<D> Serialize for PluginCallResponse<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WirePluginCallResponse::from(self).serialize(serializer)
    }
}

impl<'de, D> Deserialize<'de> for PluginCallResponse<D>
where
    D: Deserialize<'de>,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: Deserializer<'de>,
    {
        Ok(WirePluginCallResponseDe::deserialize(deserializer)?.into())
    }
}

impl Serialize for Ordering {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        WireOrdering::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ordering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(WireOrdering::deserialize(deserializer)?.into())
    }
}
