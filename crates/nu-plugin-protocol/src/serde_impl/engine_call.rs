//! Serde mapping for plugin protocol (`engine_call`).

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
