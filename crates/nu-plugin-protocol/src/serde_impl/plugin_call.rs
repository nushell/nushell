//! Serde mapping for plugin protocol (`plugin_call`).

#![allow(unused_imports)]

use crate::{
    ByteStreamInfo, CallInfo, CustomValueOp, DynamicCompletionCall, EngineCall, EngineCallResponse,
    EvaluatedCall, Feature, GetCompletionArgType, GetCompletionInfo, ListStreamInfo, Ordering,
    PipelineDataHeader, PluginCall, PluginCallResponse, PluginCustomValue, PluginInput,
    PluginOption, PluginOutput, Protocol, ProtocolInfo, StreamData, StreamMessage,
};
use nu_protocol::{LabeledError, ShellError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
struct CallInfoRef<'a, D> {
    name: &'a str,
    call: &'a EvaluatedCall,
    input: &'a D,
}

#[derive(Deserialize)]
struct CallInfoDef<D> {
    name: String,
    call: EvaluatedCall,
    input: D,
}

#[derive(Serialize)]
enum GetCompletionArgTypeRef<'a> {
    Flag(&'a str),
    Positional(usize),
}

#[derive(Deserialize)]
enum GetCompletionArgTypeDef {
    Flag(String),
    Positional(usize),
}

#[derive(Serialize)]
struct DynamicCompletionCallRef<'a> {
    call: &'a nu_protocol::ast::Call,
    strip: bool,
    pos: usize,
}

#[derive(Deserialize)]
struct DynamicCompletionCallDef {
    call: nu_protocol::ast::Call,
    strip: bool,
    pos: usize,
}

#[derive(Serialize)]
struct GetCompletionInfoRef<'a> {
    name: &'a str,
    arg_type: &'a GetCompletionArgType,
    call: &'a DynamicCompletionCall,
}

#[derive(Deserialize)]
struct GetCompletionInfoDef {
    name: String,
    arg_type: GetCompletionArgType,
    call: DynamicCompletionCall,
}

#[derive(Serialize)]
enum PluginCallRef<'a, D> {
    Metadata,
    Signature,
    Run(&'a CallInfo<D>),
    GetCompletion(&'a GetCompletionInfo),
    CustomValueOp(
        &'a nu_protocol::Spanned<PluginCustomValue>,
        &'a CustomValueOp,
    ),
}

#[derive(Deserialize)]
enum PluginCallDef<D> {
    Metadata,
    Signature,
    Run(CallInfo<D>),
    GetCompletion(GetCompletionInfo),
    CustomValueOp(nu_protocol::Spanned<PluginCustomValue>, CustomValueOp),
}

#[derive(Serialize)]
enum CustomValueOpRef<'a> {
    ToBaseValue,
    FollowPathInt {
        index: &'a nu_protocol::Spanned<usize>,
        optional: bool,
    },
    FollowPathString {
        column_name: &'a nu_protocol::Spanned<String>,
        optional: bool,
        casing: nu_protocol::casing::Casing,
    },
    PartialCmp(&'a nu_protocol::Value),
    Operation(
        &'a nu_protocol::Spanned<nu_protocol::ast::Operator>,
        &'a nu_protocol::Value,
    ),
    Save {
        path: &'a nu_protocol::Spanned<std::path::PathBuf>,
        save_call_span: nu_protocol::Span,
    },
    Dropped,
}

#[derive(Deserialize)]
enum CustomValueOpDef {
    ToBaseValue,
    FollowPathInt {
        index: nu_protocol::Spanned<usize>,
        optional: bool,
    },
    FollowPathString {
        column_name: nu_protocol::Spanned<String>,
        optional: bool,
        casing: nu_protocol::casing::Casing,
    },
    PartialCmp(nu_protocol::Value),
    Operation(
        nu_protocol::Spanned<nu_protocol::ast::Operator>,
        nu_protocol::Value,
    ),
    Save {
        path: nu_protocol::Spanned<std::path::PathBuf>,
        save_call_span: nu_protocol::Span,
    },
    Dropped,
}

#[derive(Serialize)]
enum PluginCallResponseRef<'a, D> {
    Ok,
    Error(&'a ShellError),
    Metadata(&'a nu_protocol::PluginMetadata),
    Signature(&'a Vec<nu_protocol::PluginSignature>),
    Ordering(Option<Ordering>),
    CompletionItems(&'a Option<Vec<nu_protocol::DynamicSuggestion>>),
    PipelineData(&'a D),
}

#[derive(Deserialize)]
enum PluginCallResponseDef<D> {
    Ok,
    Error(ShellError),
    Metadata(nu_protocol::PluginMetadata),
    Signature(Vec<nu_protocol::PluginSignature>),
    Ordering(Option<Ordering>),
    CompletionItems(Option<Vec<nu_protocol::DynamicSuggestion>>),
    PipelineData(D),
}

#[derive(Serialize)]
enum OrderingRef {
    Less,
    Equal,
    Greater,
}

#[derive(Deserialize)]
enum OrderingDef {
    Less,
    Equal,
    Greater,
}

impl<D> Serialize for CallInfo<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CallInfoRef {
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
        let def = CallInfoDef::deserialize(deserializer)?;
        Ok(Self {
            name: def.name,
            call: def.call,
            input: def.input,
        })
    }
}

impl Serialize for GetCompletionArgType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Flag(name) => GetCompletionArgTypeRef::Flag(name).serialize(serializer),
            Self::Positional(index) => {
                GetCompletionArgTypeRef::Positional(*index).serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for GetCompletionArgType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match GetCompletionArgTypeDef::deserialize(deserializer)? {
            GetCompletionArgTypeDef::Flag(name) => Self::Flag(name),
            GetCompletionArgTypeDef::Positional(index) => Self::Positional(index),
        })
    }
}

impl Serialize for DynamicCompletionCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        DynamicCompletionCallRef {
            call: &self.call,
            strip: self.strip,
            pos: self.pos,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DynamicCompletionCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = DynamicCompletionCallDef::deserialize(deserializer)?;
        Ok(Self {
            call: def.call,
            strip: def.strip,
            pos: def.pos,
        })
    }
}

impl Serialize for GetCompletionInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        GetCompletionInfoRef {
            name: &self.name,
            arg_type: &self.arg_type,
            call: &self.call,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GetCompletionInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = GetCompletionInfoDef::deserialize(deserializer)?;
        Ok(Self {
            name: def.name,
            arg_type: def.arg_type,
            call: def.call,
        })
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
        match self {
            Self::Metadata => PluginCallRef::<D>::Metadata.serialize(serializer),
            Self::Signature => PluginCallRef::<D>::Signature.serialize(serializer),
            Self::Run(call) => PluginCallRef::<D>::Run(call).serialize(serializer),
            Self::GetCompletion(info) => {
                PluginCallRef::<D>::GetCompletion(info).serialize(serializer)
            }
            Self::CustomValueOp(value, op) => {
                PluginCallRef::<D>::CustomValueOp(value, op).serialize(serializer)
            }
        }
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
        Ok(match PluginCallDef::deserialize(deserializer)? {
            PluginCallDef::Metadata => Self::Metadata,
            PluginCallDef::Signature => Self::Signature,
            PluginCallDef::Run(call) => Self::Run(call),
            PluginCallDef::GetCompletion(info) => Self::GetCompletion(info),
            PluginCallDef::CustomValueOp(value, op) => Self::CustomValueOp(value, op),
        })
    }
}

impl Serialize for CustomValueOp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::ToBaseValue => CustomValueOpRef::ToBaseValue.serialize(serializer),
            Self::FollowPathInt { index, optional } => CustomValueOpRef::FollowPathInt {
                index,
                optional: *optional,
            }
            .serialize(serializer),
            Self::FollowPathString {
                column_name,
                optional,
                casing,
            } => CustomValueOpRef::FollowPathString {
                column_name,
                optional: *optional,
                casing: *casing,
            }
            .serialize(serializer),
            Self::PartialCmp(value) => CustomValueOpRef::PartialCmp(value).serialize(serializer),
            Self::Operation(operator, value) => {
                CustomValueOpRef::Operation(operator, value).serialize(serializer)
            }
            Self::Save {
                path,
                save_call_span,
            } => CustomValueOpRef::Save {
                path,
                save_call_span: *save_call_span,
            }
            .serialize(serializer),
            Self::Dropped => CustomValueOpRef::Dropped.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for CustomValueOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match CustomValueOpDef::deserialize(deserializer)? {
            CustomValueOpDef::ToBaseValue => Self::ToBaseValue,
            CustomValueOpDef::FollowPathInt { index, optional } => {
                Self::FollowPathInt { index, optional }
            }
            CustomValueOpDef::FollowPathString {
                column_name,
                optional,
                casing,
            } => Self::FollowPathString {
                column_name,
                optional,
                casing,
            },
            CustomValueOpDef::PartialCmp(value) => Self::PartialCmp(value),
            CustomValueOpDef::Operation(operator, value) => Self::Operation(operator, value),
            CustomValueOpDef::Save {
                path,
                save_call_span,
            } => Self::Save {
                path,
                save_call_span,
            },
            CustomValueOpDef::Dropped => Self::Dropped,
        })
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
        match self {
            Self::Ok => PluginCallResponseRef::<D>::Ok.serialize(serializer),
            Self::Error(err) => PluginCallResponseRef::<D>::Error(err).serialize(serializer),
            Self::Metadata(meta) => {
                PluginCallResponseRef::<D>::Metadata(meta).serialize(serializer)
            }
            Self::Signature(sigs) => {
                PluginCallResponseRef::<D>::Signature(sigs).serialize(serializer)
            }
            Self::Ordering(ordering) => {
                PluginCallResponseRef::<D>::Ordering(*ordering).serialize(serializer)
            }
            Self::CompletionItems(items) => {
                PluginCallResponseRef::<D>::CompletionItems(items).serialize(serializer)
            }
            Self::PipelineData(data) => {
                PluginCallResponseRef::<D>::PipelineData(data).serialize(serializer)
            }
        }
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
        Ok(match PluginCallResponseDef::deserialize(deserializer)? {
            PluginCallResponseDef::Ok => Self::Ok,
            PluginCallResponseDef::Error(err) => Self::Error(err),
            PluginCallResponseDef::Metadata(meta) => Self::Metadata(meta),
            PluginCallResponseDef::Signature(sigs) => Self::Signature(sigs),
            PluginCallResponseDef::Ordering(ordering) => Self::Ordering(ordering),
            PluginCallResponseDef::CompletionItems(items) => Self::CompletionItems(items),
            PluginCallResponseDef::PipelineData(data) => Self::PipelineData(data),
        })
    }
}

impl Serialize for Ordering {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Less => OrderingRef::Less.serialize(serializer),
            Self::Equal => OrderingRef::Equal.serialize(serializer),
            Self::Greater => OrderingRef::Greater.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Ordering {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match OrderingDef::deserialize(deserializer)? {
            OrderingDef::Less => Self::Less,
            OrderingDef::Equal => Self::Equal,
            OrderingDef::Greater => Self::Greater,
        })
    }
}
