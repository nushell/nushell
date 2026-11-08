//! Compile-time witnesses for the messages of the plugin protocol's frozen tier.
//!
//! This is the counterpart of `nu_protocol`'s `plugin/wire_witness.rs` for the types defined in
//! this crate. Each witness names every variant and field of one message type, so the build
//! stops here when one is added, removed, or renamed. When that happens, pick the protocol bump
//! from the table in the [`PLUGIN_PROTOCOL_VERSION`](crate::PLUGIN_PROTOCOL_VERSION) docs, then
//! update the witness, add the new message to the corpus in `src/tests.rs`, and regenerate the
//! snapshots.
//!
//! Every field is matched as `_`, so changing a field's type or a serde attribute still compiles.
//! Only the snapshots catch that. `PluginCustomValue` has its witness next to its private fields,
//! and the payloads of `GetCompletion` (`GetCompletionInfo` and the parsed call in it) are
//! engine-internal, so they have none.
//!
//! Nothing here runs. Each witness is a non-capturing closure coerced to a function pointer, so
//! it is type-checked and then discarded. The generic messages are checked with `()` as their
//! data type, which doesn't change their variants.

use crate::{
    ByteStreamInfo, CallInfo, CustomValueOp, EngineCall, EngineCallResponse, EvaluatedCall,
    Feature, ListStreamInfo, Ordering, PipelineDataHeader, PluginCall, PluginCallResponse,
    PluginInput, PluginOption, PluginOutput, Protocol, ProtocolInfo, StreamData,
};

const _: fn(&PluginInput) = |input| match input {
    PluginInput::Hello(_)
    | PluginInput::Call(_, _)
    | PluginInput::Goodbye
    | PluginInput::EngineCallResponse(_, _)
    | PluginInput::Data(_, _)
    | PluginInput::End(_)
    | PluginInput::Drop(_)
    | PluginInput::Ack(_)
    | PluginInput::Signal(_) => {}
};

const _: fn(&PluginOutput) = |output| match output {
    PluginOutput::Hello(_)
    | PluginOutput::Option(_)
    | PluginOutput::CallResponse(_, _)
    | PluginOutput::EngineCall {
        context: _,
        id: _,
        call: _,
    }
    | PluginOutput::Data(_, _)
    | PluginOutput::End(_)
    | PluginOutput::Drop(_)
    | PluginOutput::Ack(_) => {}
};

const _: fn(&ProtocolInfo) = |info| {
    let ProtocolInfo {
        protocol: _,
        version: _,
        nushell_version: _,
        features: _,
    } = info;
};

const _: fn(&Protocol) = |protocol| match protocol {
    Protocol::NuPlugin => {}
};

const _: fn(&Feature) = |feature| match feature {
    Feature::LocalSocket | Feature::Unknown => {}
};

const _: fn(&PluginCall<()>) = |call| match call {
    PluginCall::Metadata
    | PluginCall::Signature
    | PluginCall::Run(_)
    | PluginCall::GetCompletion(_)
    | PluginCall::CustomValueOp(_, _) => {}
};

const _: fn(&CallInfo<()>) = |info| {
    let CallInfo {
        name: _,
        call: _,
        input: _,
    } = info;
};

const _: fn(&EvaluatedCall) = |call| {
    let EvaluatedCall {
        head: _,
        positional: _,
        named: _,
    } = call;
};

const _: fn(&CustomValueOp) = |op| match op {
    CustomValueOp::ToBaseValue
    | CustomValueOp::FollowPathInt {
        index: _,
        optional: _,
    }
    | CustomValueOp::FollowPathString {
        column_name: _,
        optional: _,
        casing: _,
    }
    | CustomValueOp::PartialCmp(_)
    | CustomValueOp::Operation(_, _)
    | CustomValueOp::Save {
        path: _,
        save_call_span: _,
    }
    | CustomValueOp::Dropped => {}
};

const _: fn(&PluginCallResponse<()>) = |response| match response {
    PluginCallResponse::Ok
    | PluginCallResponse::Error(_)
    | PluginCallResponse::Metadata(_)
    | PluginCallResponse::Signature(_)
    | PluginCallResponse::Ordering(_)
    | PluginCallResponse::CompletionItems(_)
    | PluginCallResponse::PipelineData(_) => {}
};

const _: fn(&Ordering) = |ordering| match ordering {
    Ordering::Less | Ordering::Equal | Ordering::Greater => {}
};

const _: fn(&PluginOption) = |option| match option {
    PluginOption::GcDisabled(_) => {}
};

const _: fn(&EngineCall<()>) = |call| match call {
    EngineCall::GetConfig
    | EngineCall::GetPluginConfig
    | EngineCall::GetEnvVar(_)
    | EngineCall::GetEnvVars
    | EngineCall::GetCurrentDir
    | EngineCall::AddEnvVar(_, _)
    | EngineCall::GetHelp
    | EngineCall::EnterForeground
    | EngineCall::LeaveForeground
    | EngineCall::GetSpanContents(_)
    | EngineCall::EvalClosure {
        closure: _,
        positional: _,
        input: _,
        redirect_stdout: _,
        redirect_stderr: _,
    }
    | EngineCall::FindDecl(_)
    | EngineCall::GetBlockIR(_)
    | EngineCall::CallDecl {
        decl_id: _,
        call: _,
        input: _,
        redirect_stdout: _,
        redirect_stderr: _,
    } => {}
};

const _: fn(&EngineCallResponse<()>) = |response| match response {
    EngineCallResponse::Error(_)
    | EngineCallResponse::PipelineData(_)
    | EngineCallResponse::Config(_)
    | EngineCallResponse::ValueMap(_)
    | EngineCallResponse::Identifier(_)
    | EngineCallResponse::IrBlock(_) => {}
};

const _: fn(&PipelineDataHeader) = |header| match header {
    PipelineDataHeader::Empty
    | PipelineDataHeader::Value(_, _)
    | PipelineDataHeader::ListStream(_)
    | PipelineDataHeader::ByteStream(_) => {}
};

const _: fn(&ListStreamInfo) = |info| {
    let ListStreamInfo {
        id: _,
        span: _,
        metadata: _,
    } = info;
};

const _: fn(&ByteStreamInfo) = |info| {
    let ByteStreamInfo {
        id: _,
        span: _,
        type_: _,
        metadata: _,
    } = info;
};

const _: fn(&StreamData) = |data| match data {
    StreamData::List(_) | StreamData::Raw(_) => {}
};
