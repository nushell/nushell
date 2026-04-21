//! Explicit serde implementation for plugin protocol types.
//!
//! This module intentionally maps public protocol types to private serde helper
//! representations so protocol wire changes are explicit in review.

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
enum PipelineDataHeaderRef<'a> {
    Empty,
    Value(
        &'a nu_protocol::Value,
        &'a Option<nu_protocol::PipelineMetadata>,
    ),
    ListStream(&'a ListStreamInfo),
    ByteStream(&'a ByteStreamInfo),
}

#[derive(Deserialize)]
enum PipelineDataHeaderDef {
    Empty,
    Value(nu_protocol::Value, Option<nu_protocol::PipelineMetadata>),
    ListStream(ListStreamInfo),
    ByteStream(ByteStreamInfo),
}

#[derive(Serialize)]
struct ListStreamInfoRef {
    id: usize,
    span: nu_protocol::Span,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Deserialize)]
struct ListStreamInfoDef {
    id: usize,
    span: nu_protocol::Span,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Serialize)]
struct ByteStreamInfoRef {
    id: usize,
    span: nu_protocol::Span,
    #[serde(rename = "type")]
    type_: nu_protocol::ByteStreamType,
    metadata: Option<nu_protocol::PipelineMetadata>,
}

#[derive(Deserialize)]
struct ByteStreamInfoDef {
    id: usize,
    span: nu_protocol::Span,
    #[serde(rename = "type")]
    type_: nu_protocol::ByteStreamType,
    metadata: Option<nu_protocol::PipelineMetadata>,
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
enum PluginInputRef<'a> {
    Hello(&'a ProtocolInfo),
    Call(usize, &'a PluginCall<PipelineDataHeader>),
    Goodbye,
    EngineCallResponse(usize, &'a EngineCallResponse<PipelineDataHeader>),
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
    Signal(nu_protocol::SignalAction),
}

#[derive(Deserialize)]
enum PluginInputDef {
    Hello(ProtocolInfo),
    Call(usize, PluginCall<PipelineDataHeader>),
    Goodbye,
    EngineCallResponse(usize, EngineCallResponse<PipelineDataHeader>),
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
    Signal(nu_protocol::SignalAction),
}

#[derive(Serialize)]
enum StreamDataRef<'a> {
    List(&'a nu_protocol::Value),
    Raw(&'a Result<Vec<u8>, LabeledError>),
}

#[derive(Deserialize)]
enum StreamDataDef {
    List(nu_protocol::Value),
    Raw(Result<Vec<u8>, LabeledError>),
}

#[derive(Serialize)]
enum StreamMessageRef<'a> {
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Deserialize)]
enum StreamMessageDef {
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
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
enum PluginOptionRef {
    GcDisabled(bool),
}

#[derive(Deserialize)]
enum PluginOptionDef {
    GcDisabled(bool),
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

#[derive(Serialize)]
enum PluginOutputRef<'a> {
    Hello(&'a ProtocolInfo),
    Option(&'a PluginOption),
    CallResponse(usize, &'a PluginCallResponse<PipelineDataHeader>),
    EngineCall {
        context: usize,
        id: usize,
        call: &'a EngineCall<PipelineDataHeader>,
    },
    Data(usize, &'a StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Deserialize)]
enum PluginOutputDef {
    Hello(ProtocolInfo),
    Option(PluginOption),
    CallResponse(usize, PluginCallResponse<PipelineDataHeader>),
    EngineCall {
        context: usize,
        id: usize,
        call: EngineCall<PipelineDataHeader>,
    },
    Data(usize, StreamData),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Serialize)]
enum EngineCallRef<'a, D> {
    GetConfig,
    GetPluginConfig,
    GetEnvVar(&'a str),
    GetEnvVars,
    GetCurrentDir,
    AddEnvVar(&'a str, &'a nu_protocol::Value),
    GetHelp,
    EnterForeground,
    LeaveForeground,
    GetSpanContents(nu_protocol::Span),
    EvalClosure {
        closure: &'a nu_protocol::Spanned<nu_protocol::engine::Closure>,
        positional: &'a Vec<nu_protocol::Value>,
        input: &'a D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
    FindDecl(&'a str),
    GetBlockIR(nu_protocol::BlockId),
    CallDecl {
        decl_id: nu_protocol::DeclId,
        call: &'a EvaluatedCall,
        input: &'a D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
}

#[derive(Deserialize)]
enum EngineCallDef<D> {
    GetConfig,
    GetPluginConfig,
    GetEnvVar(String),
    GetEnvVars,
    GetCurrentDir,
    AddEnvVar(String, nu_protocol::Value),
    GetHelp,
    EnterForeground,
    LeaveForeground,
    GetSpanContents(nu_protocol::Span),
    EvalClosure {
        closure: nu_protocol::Spanned<nu_protocol::engine::Closure>,
        positional: Vec<nu_protocol::Value>,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
    FindDecl(String),
    GetBlockIR(nu_protocol::BlockId),
    CallDecl {
        decl_id: nu_protocol::DeclId,
        call: EvaluatedCall,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
}

#[derive(Serialize)]
enum EngineCallResponseRef<'a, D> {
    Error(&'a ShellError),
    PipelineData(&'a D),
    Config(&'a nu_utils::SharedCow<nu_protocol::Config>),
    ValueMap(&'a std::collections::HashMap<String, nu_protocol::Value>),
    Identifier(nu_protocol::DeclId),
    IrBlock(&'a nu_protocol::ir::IrBlock),
}

#[derive(Deserialize)]
enum EngineCallResponseDef<D> {
    Error(ShellError),
    PipelineData(D),
    Config(nu_utils::SharedCow<nu_protocol::Config>),
    ValueMap(std::collections::HashMap<String, nu_protocol::Value>),
    Identifier(nu_protocol::DeclId),
    IrBlock(Box<nu_protocol::ir::IrBlock>),
}

#[derive(Serialize)]
struct EvaluatedCallRef<'a> {
    head: nu_protocol::Span,
    positional: &'a Vec<nu_protocol::Value>,
    named: &'a Vec<(nu_protocol::Spanned<String>, Option<nu_protocol::Value>)>,
}

#[derive(Deserialize)]
struct EvaluatedCallDef {
    head: nu_protocol::Span,
    positional: Vec<nu_protocol::Value>,
    named: Vec<(nu_protocol::Spanned<String>, Option<nu_protocol::Value>)>,
}

#[derive(Serialize)]
struct PluginCustomValueRef<'a> {
    name: &'a str,
    data: &'a [u8],
    #[serde(default, skip_serializing_if = "is_false")]
    notify_on_drop: bool,
}

#[derive(Deserialize)]
struct PluginCustomValueDef {
    name: String,
    data: Vec<u8>,
    #[serde(default)]
    notify_on_drop: bool,
}

#[derive(Serialize)]
struct ProtocolInfoRef<'a> {
    protocol: &'a Protocol,
    version: &'a str,
    features: &'a Vec<Feature>,
}

#[derive(Deserialize)]
struct ProtocolInfoDef {
    protocol: Protocol,
    version: String,
    features: Vec<Feature>,
}

#[derive(Serialize)]
enum ProtocolRef {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

#[derive(Deserialize)]
enum ProtocolDef {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

#[derive(Serialize)]
#[serde(tag = "name")]
enum FeatureRef {
    LocalSocket,
}

#[derive(Deserialize)]
#[serde(tag = "name")]
enum FeatureDef {
    LocalSocket,
    #[serde(other)]
    Unknown,
}

fn is_false(value: &bool) -> bool {
    !value
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

impl Serialize for PipelineDataHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Empty => PipelineDataHeaderRef::Empty.serialize(serializer),
            Self::Value(value, metadata) => {
                PipelineDataHeaderRef::Value(value, metadata).serialize(serializer)
            }
            Self::ListStream(info) => PipelineDataHeaderRef::ListStream(info).serialize(serializer),
            Self::ByteStream(info) => PipelineDataHeaderRef::ByteStream(info).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PipelineDataHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PipelineDataHeaderDef::deserialize(deserializer)? {
            PipelineDataHeaderDef::Empty => Self::Empty,
            PipelineDataHeaderDef::Value(value, metadata) => Self::Value(value, metadata),
            PipelineDataHeaderDef::ListStream(info) => Self::ListStream(info),
            PipelineDataHeaderDef::ByteStream(info) => Self::ByteStream(info),
        })
    }
}

impl Serialize for ListStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ListStreamInfoRef {
            id: self.id,
            span: self.span,
            metadata: self.metadata.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ListStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ListStreamInfoDef::deserialize(deserializer)?;
        Ok(Self {
            id: def.id,
            span: def.span,
            metadata: def.metadata,
        })
    }
}

impl Serialize for ByteStreamInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ByteStreamInfoRef {
            id: self.id,
            span: self.span,
            type_: self.type_,
            metadata: self.metadata.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ByteStreamInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ByteStreamInfoDef::deserialize(deserializer)?;
        Ok(Self {
            id: def.id,
            span: def.span,
            type_: def.type_,
            metadata: def.metadata,
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

impl Serialize for PluginInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Hello(info) => PluginInputRef::Hello(info).serialize(serializer),
            Self::Call(id, call) => PluginInputRef::Call(*id, call).serialize(serializer),
            Self::Goodbye => PluginInputRef::Goodbye.serialize(serializer),
            Self::EngineCallResponse(id, response) => {
                PluginInputRef::EngineCallResponse(*id, response).serialize(serializer)
            }
            Self::Data(id, data) => PluginInputRef::Data(*id, data).serialize(serializer),
            Self::End(id) => PluginInputRef::End(*id).serialize(serializer),
            Self::Drop(id) => PluginInputRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => PluginInputRef::Ack(*id).serialize(serializer),
            Self::Signal(signal) => PluginInputRef::Signal(*signal).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginInputDef::deserialize(deserializer)? {
            PluginInputDef::Hello(info) => Self::Hello(info),
            PluginInputDef::Call(id, call) => Self::Call(id, call),
            PluginInputDef::Goodbye => Self::Goodbye,
            PluginInputDef::EngineCallResponse(id, response) => {
                Self::EngineCallResponse(id, response)
            }
            PluginInputDef::Data(id, data) => Self::Data(id, data),
            PluginInputDef::End(id) => Self::End(id),
            PluginInputDef::Drop(id) => Self::Drop(id),
            PluginInputDef::Ack(id) => Self::Ack(id),
            PluginInputDef::Signal(signal) => Self::Signal(signal),
        })
    }
}

impl Serialize for StreamData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::List(value) => StreamDataRef::List(value).serialize(serializer),
            Self::Raw(value) => StreamDataRef::Raw(value).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StreamData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match StreamDataDef::deserialize(deserializer)? {
            StreamDataDef::List(value) => Self::List(value),
            StreamDataDef::Raw(value) => Self::Raw(value),
        })
    }
}

impl Serialize for StreamMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Data(id, data) => StreamMessageRef::Data(*id, data).serialize(serializer),
            Self::End(id) => StreamMessageRef::End(*id).serialize(serializer),
            Self::Drop(id) => StreamMessageRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => StreamMessageRef::Ack(*id).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for StreamMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match StreamMessageDef::deserialize(deserializer)? {
            StreamMessageDef::Data(id, data) => Self::Data(id, data),
            StreamMessageDef::End(id) => Self::End(id),
            StreamMessageDef::Drop(id) => Self::Drop(id),
            StreamMessageDef::Ack(id) => Self::Ack(id),
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

impl Serialize for PluginOption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::GcDisabled(value) => PluginOptionRef::GcDisabled(*value).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginOptionDef::deserialize(deserializer)? {
            PluginOptionDef::GcDisabled(value) => Self::GcDisabled(value),
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

impl Serialize for PluginOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Hello(info) => PluginOutputRef::Hello(info).serialize(serializer),
            Self::Option(option) => PluginOutputRef::Option(option).serialize(serializer),
            Self::CallResponse(id, response) => {
                PluginOutputRef::CallResponse(*id, response).serialize(serializer)
            }
            Self::EngineCall { context, id, call } => PluginOutputRef::EngineCall {
                context: *context,
                id: *id,
                call,
            }
            .serialize(serializer),
            Self::Data(id, data) => PluginOutputRef::Data(*id, data).serialize(serializer),
            Self::End(id) => PluginOutputRef::End(*id).serialize(serializer),
            Self::Drop(id) => PluginOutputRef::Drop(*id).serialize(serializer),
            Self::Ack(id) => PluginOutputRef::Ack(*id).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for PluginOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match PluginOutputDef::deserialize(deserializer)? {
            PluginOutputDef::Hello(info) => Self::Hello(info),
            PluginOutputDef::Option(option) => Self::Option(option),
            PluginOutputDef::CallResponse(id, response) => Self::CallResponse(id, response),
            PluginOutputDef::EngineCall { context, id, call } => {
                Self::EngineCall { context, id, call }
            }
            PluginOutputDef::Data(id, data) => Self::Data(id, data),
            PluginOutputDef::End(id) => Self::End(id),
            PluginOutputDef::Drop(id) => Self::Drop(id),
            PluginOutputDef::Ack(id) => Self::Ack(id),
        })
    }
}

impl<D> Serialize for EngineCall<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::GetConfig => EngineCallRef::<D>::GetConfig.serialize(serializer),
            Self::GetPluginConfig => EngineCallRef::<D>::GetPluginConfig.serialize(serializer),
            Self::GetEnvVar(name) => EngineCallRef::<D>::GetEnvVar(name).serialize(serializer),
            Self::GetEnvVars => EngineCallRef::<D>::GetEnvVars.serialize(serializer),
            Self::GetCurrentDir => EngineCallRef::<D>::GetCurrentDir.serialize(serializer),
            Self::AddEnvVar(name, value) => {
                EngineCallRef::<D>::AddEnvVar(name, value).serialize(serializer)
            }
            Self::GetHelp => EngineCallRef::<D>::GetHelp.serialize(serializer),
            Self::EnterForeground => EngineCallRef::<D>::EnterForeground.serialize(serializer),
            Self::LeaveForeground => EngineCallRef::<D>::LeaveForeground.serialize(serializer),
            Self::GetSpanContents(span) => {
                EngineCallRef::<D>::GetSpanContents(*span).serialize(serializer)
            }
            Self::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => EngineCallRef::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout: *redirect_stdout,
                redirect_stderr: *redirect_stderr,
            }
            .serialize(serializer),
            Self::FindDecl(name) => EngineCallRef::<D>::FindDecl(name).serialize(serializer),
            Self::GetBlockIR(block_id) => {
                EngineCallRef::<D>::GetBlockIR(*block_id).serialize(serializer)
            }
            Self::CallDecl {
                decl_id,
                call,
                input,
                redirect_stdout,
                redirect_stderr,
            } => EngineCallRef::CallDecl {
                decl_id: *decl_id,
                call,
                input,
                redirect_stdout: *redirect_stdout,
                redirect_stderr: *redirect_stderr,
            }
            .serialize(serializer),
        }
    }
}

impl<'de, D> Deserialize<'de> for EngineCall<D>
where
    D: Deserialize<'de>,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: Deserializer<'de>,
    {
        Ok(match EngineCallDef::deserialize(deserializer)? {
            EngineCallDef::GetConfig => Self::GetConfig,
            EngineCallDef::GetPluginConfig => Self::GetPluginConfig,
            EngineCallDef::GetEnvVar(name) => Self::GetEnvVar(name),
            EngineCallDef::GetEnvVars => Self::GetEnvVars,
            EngineCallDef::GetCurrentDir => Self::GetCurrentDir,
            EngineCallDef::AddEnvVar(name, value) => Self::AddEnvVar(name, value),
            EngineCallDef::GetHelp => Self::GetHelp,
            EngineCallDef::EnterForeground => Self::EnterForeground,
            EngineCallDef::LeaveForeground => Self::LeaveForeground,
            EngineCallDef::GetSpanContents(span) => Self::GetSpanContents(span),
            EngineCallDef::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => Self::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            },
            EngineCallDef::FindDecl(name) => Self::FindDecl(name),
            EngineCallDef::GetBlockIR(block_id) => Self::GetBlockIR(block_id),
            EngineCallDef::CallDecl {
                decl_id,
                call,
                input,
                redirect_stdout,
                redirect_stderr,
            } => Self::CallDecl {
                decl_id,
                call,
                input,
                redirect_stdout,
                redirect_stderr,
            },
        })
    }
}

impl<D> Serialize for EngineCallResponse<D>
where
    D: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Error(err) => EngineCallResponseRef::<D>::Error(err).serialize(serializer),
            Self::PipelineData(data) => {
                EngineCallResponseRef::<D>::PipelineData(data).serialize(serializer)
            }
            Self::Config(config) => {
                EngineCallResponseRef::<D>::Config(config).serialize(serializer)
            }
            Self::ValueMap(map) => EngineCallResponseRef::<D>::ValueMap(map).serialize(serializer),
            Self::Identifier(id) => {
                EngineCallResponseRef::<D>::Identifier(*id).serialize(serializer)
            }
            Self::IrBlock(ir) => {
                EngineCallResponseRef::<D>::IrBlock(ir.as_ref()).serialize(serializer)
            }
        }
    }
}

impl<'de, D> Deserialize<'de> for EngineCallResponse<D>
where
    D: Deserialize<'de>,
{
    fn deserialize<T>(deserializer: T) -> Result<Self, T::Error>
    where
        T: Deserializer<'de>,
    {
        Ok(match EngineCallResponseDef::deserialize(deserializer)? {
            EngineCallResponseDef::Error(err) => Self::Error(err),
            EngineCallResponseDef::PipelineData(data) => Self::PipelineData(data),
            EngineCallResponseDef::Config(config) => Self::Config(config),
            EngineCallResponseDef::ValueMap(map) => Self::ValueMap(map),
            EngineCallResponseDef::Identifier(id) => Self::Identifier(id),
            EngineCallResponseDef::IrBlock(ir) => Self::IrBlock(ir),
        })
    }
}

impl Serialize for EvaluatedCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        EvaluatedCallRef {
            head: self.head,
            positional: &self.positional,
            named: &self.named,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EvaluatedCall {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = EvaluatedCallDef::deserialize(deserializer)?;
        Ok(Self {
            head: def.head,
            positional: def.positional,
            named: def.named,
        })
    }
}

impl Serialize for PluginCustomValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PluginCustomValueRef {
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
        let def = PluginCustomValueDef::deserialize(deserializer)?;
        Ok(Self::new(def.name, def.data, def.notify_on_drop))
    }
}

impl Serialize for ProtocolInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ProtocolInfoRef {
            protocol: &self.protocol,
            version: &self.version,
            features: &self.features,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProtocolInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = ProtocolInfoDef::deserialize(deserializer)?;
        Ok(Self {
            protocol: def.protocol,
            version: def.version,
            features: def.features,
        })
    }
}

impl Serialize for Protocol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::NuPlugin => ProtocolRef::NuPlugin.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Protocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match ProtocolDef::deserialize(deserializer)? {
            ProtocolDef::NuPlugin => Self::NuPlugin,
        })
    }
}

impl Serialize for Feature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::LocalSocket => FeatureRef::LocalSocket.serialize(serializer),
            Self::Unknown => Err(serde::ser::Error::custom(
                "cannot serialize unknown protocol feature",
            )),
        }
    }
}

impl<'de> Deserialize<'de> for Feature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match FeatureDef::deserialize(deserializer)? {
            FeatureDef::LocalSocket => Self::LocalSocket,
            FeatureDef::Unknown => Self::Unknown,
        })
    }
}
