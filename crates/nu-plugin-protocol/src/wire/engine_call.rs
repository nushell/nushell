//! Wire mapping for engine calls and engine call responses.

use crate::{EngineCall, EngineCallResponse, EvaluatedCall};
use nu_protocol::{
    BlockId, Config, DeclId, ShellError, Span, Spanned, Value, engine::Closure, ir::IrBlock,
};
use nu_utils::SharedCow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use super::value::WireValue;

#[derive(Serialize)]
enum WireEngineCall<'a, D: Serialize> {
    GetConfig,
    GetPluginConfig,
    GetEnvVar(&'a str),
    GetEnvVars,
    GetCurrentDir,
    AddEnvVar(&'a str, WireValue),
    GetHelp,
    EnterForeground,
    LeaveForeground,
    GetSpanContents(Span),
    EvalClosure {
        closure: &'a Spanned<Closure>,
        positional: Vec<WireValue>,
        input: &'a D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
    FindDecl(&'a str),
    GetBlockIR(BlockId),
    CallDecl {
        decl_id: DeclId,
        call: &'a EvaluatedCall,
        input: &'a D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
}

#[derive(Deserialize)]
enum WireEngineCallDe<D> {
    GetConfig,
    GetPluginConfig,
    GetEnvVar(String),
    GetEnvVars,
    GetCurrentDir,
    AddEnvVar(String, WireValue),
    GetHelp,
    EnterForeground,
    LeaveForeground,
    GetSpanContents(Span),
    EvalClosure {
        closure: Spanned<Closure>,
        positional: Vec<WireValue>,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
    FindDecl(String),
    GetBlockIR(BlockId),
    CallDecl {
        decl_id: DeclId,
        call: EvaluatedCall,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
}

impl<'a, D: Serialize> From<&'a EngineCall<D>> for WireEngineCall<'a, D> {
    fn from(call: &'a EngineCall<D>) -> Self {
        match call {
            EngineCall::GetConfig => Self::GetConfig,
            EngineCall::GetPluginConfig => Self::GetPluginConfig,
            EngineCall::GetEnvVar(name) => Self::GetEnvVar(name),
            EngineCall::GetEnvVars => Self::GetEnvVars,
            EngineCall::GetCurrentDir => Self::GetCurrentDir,
            EngineCall::AddEnvVar(name, value) => Self::AddEnvVar(name, WireValue::from(value)),
            EngineCall::GetHelp => Self::GetHelp,
            EngineCall::EnterForeground => Self::EnterForeground,
            EngineCall::LeaveForeground => Self::LeaveForeground,
            EngineCall::GetSpanContents(span) => Self::GetSpanContents(*span),
            EngineCall::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => Self::EvalClosure {
                closure,
                positional: positional.iter().map(WireValue::from).collect(),
                input,
                redirect_stdout: *redirect_stdout,
                redirect_stderr: *redirect_stderr,
            },
            EngineCall::FindDecl(name) => Self::FindDecl(name),
            EngineCall::GetBlockIR(block_id) => Self::GetBlockIR(*block_id),
            EngineCall::CallDecl {
                decl_id,
                call,
                input,
                redirect_stdout,
                redirect_stderr,
            } => Self::CallDecl {
                decl_id: *decl_id,
                call,
                input,
                redirect_stdout: *redirect_stdout,
                redirect_stderr: *redirect_stderr,
            },
        }
    }
}

impl<D> From<WireEngineCallDe<D>> for EngineCall<D> {
    fn from(wire: WireEngineCallDe<D>) -> Self {
        match wire {
            WireEngineCallDe::GetConfig => Self::GetConfig,
            WireEngineCallDe::GetPluginConfig => Self::GetPluginConfig,
            WireEngineCallDe::GetEnvVar(name) => Self::GetEnvVar(name),
            WireEngineCallDe::GetEnvVars => Self::GetEnvVars,
            WireEngineCallDe::GetCurrentDir => Self::GetCurrentDir,
            WireEngineCallDe::AddEnvVar(name, value) => Self::AddEnvVar(name, Value::from(value)),
            WireEngineCallDe::GetHelp => Self::GetHelp,
            WireEngineCallDe::EnterForeground => Self::EnterForeground,
            WireEngineCallDe::LeaveForeground => Self::LeaveForeground,
            WireEngineCallDe::GetSpanContents(span) => Self::GetSpanContents(span),
            WireEngineCallDe::EvalClosure {
                closure,
                positional,
                input,
                redirect_stdout,
                redirect_stderr,
            } => Self::EvalClosure {
                closure,
                positional: positional.into_iter().map(Value::from).collect(),
                input,
                redirect_stdout,
                redirect_stderr,
            },
            WireEngineCallDe::FindDecl(name) => Self::FindDecl(name),
            WireEngineCallDe::GetBlockIR(block_id) => Self::GetBlockIR(block_id),
            WireEngineCallDe::CallDecl {
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
        }
    }
}

#[derive(Serialize)]
enum WireEngineCallResponse<'a, D: Serialize> {
    Error(&'a ShellError),
    PipelineData(&'a D),
    Config(&'a SharedCow<Config>),
    ValueMap(HashMap<String, WireValue>),
    Identifier(DeclId),
    IrBlock(&'a IrBlock),
}

#[derive(Deserialize)]
enum WireEngineCallResponseDe<D> {
    Error(ShellError),
    PipelineData(D),
    Config(SharedCow<Config>),
    ValueMap(HashMap<String, WireValue>),
    Identifier(DeclId),
    IrBlock(Box<IrBlock>),
}

impl<'a, D: Serialize> From<&'a EngineCallResponse<D>> for WireEngineCallResponse<'a, D> {
    fn from(response: &'a EngineCallResponse<D>) -> Self {
        match response {
            EngineCallResponse::Error(err) => Self::Error(err),
            EngineCallResponse::PipelineData(data) => Self::PipelineData(data),
            EngineCallResponse::Config(config) => Self::Config(config),
            EngineCallResponse::ValueMap(map) => Self::ValueMap(
                map.iter()
                    .map(|(k, v)| (k.clone(), WireValue::from(v)))
                    .collect(),
            ),
            EngineCallResponse::Identifier(id) => Self::Identifier(*id),
            EngineCallResponse::IrBlock(ir) => Self::IrBlock(ir.as_ref()),
        }
    }
}

impl<D> From<WireEngineCallResponseDe<D>> for EngineCallResponse<D> {
    fn from(wire: WireEngineCallResponseDe<D>) -> Self {
        match wire {
            WireEngineCallResponseDe::Error(err) => Self::Error(err),
            WireEngineCallResponseDe::PipelineData(data) => Self::PipelineData(data),
            WireEngineCallResponseDe::Config(config) => Self::Config(config),
            WireEngineCallResponseDe::ValueMap(map) => {
                Self::ValueMap(map.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
            WireEngineCallResponseDe::Identifier(id) => Self::Identifier(id),
            WireEngineCallResponseDe::IrBlock(ir) => Self::IrBlock(ir),
        }
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
        WireEngineCall::from(self).serialize(serializer)
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
        Ok(WireEngineCallDe::deserialize(deserializer)?.into())
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
        WireEngineCallResponse::from(self).serialize(serializer)
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
        Ok(WireEngineCallResponseDe::deserialize(deserializer)?.into())
    }
}
