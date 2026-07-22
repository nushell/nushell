//! JSON schema generation for the plugin protocol wire format.
//!
//! The schema is derived from protocol-specific mirror types that match the
//! explicit serde mapping in this crate. Engine-internal payloads that are not
//! part of the stable plugin contract are represented as generic JSON values.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginProtocolSchemaDocument {
    pub plugin_input: PluginInputSchema,
    pub plugin_output: PluginOutputSchema,
    pub protocol_info: ProtocolInfoSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginInputSchema {
    Hello(ProtocolInfoSchema),
    Call(usize, PluginCallSchema<PipelineDataHeaderSchema>),
    Goodbye,
    EngineCallResponse(usize, EngineCallResponseSchema<PipelineDataHeaderSchema>),
    Data(usize, StreamDataSchema),
    End(usize),
    Drop(usize),
    Ack(usize),
    Signal(SignalActionSchema),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginOutputSchema {
    Hello(ProtocolInfoSchema),
    Option(PluginOptionSchema),
    CallResponse(usize, PluginCallResponseSchema<PipelineDataHeaderSchema>),
    EngineCall {
        context: usize,
        id: usize,
        call: EngineCallSchema<PipelineDataHeaderSchema>,
    },
    Data(usize, StreamDataSchema),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProtocolInfoSchema {
    pub protocol: ProtocolSchema,
    pub version: String,
    pub features: Vec<FeatureSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ProtocolSchema {
    #[serde(rename = "nu-plugin")]
    NuPlugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "name")]
pub enum FeatureSchema {
    LocalSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginOptionSchema {
    GcDisabled(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallInfoSchema<D> {
    pub name: String,
    pub call: EvaluatedCallSchema,
    pub input: D,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvaluatedCallSchema {
    pub head: SpanSchema,
    pub positional: Vec<ValueSchema>,
    pub named: Vec<(SpannedSchema<String>, Option<ValueSchema>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginCallSchema<D> {
    Metadata,
    Signature,
    Run(CallInfoSchema<D>),
    GetCompletion(GetCompletionInfoSchema),
    CustomValueOp(SpannedSchema<PluginCustomValueSchema>, CustomValueOpSchema),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCompletionInfoSchema {
    pub name: String,
    pub arg_type: GetCompletionArgTypeSchema,
    pub call: DynamicCompletionCallSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum GetCompletionArgTypeSchema {
    Flag(String),
    Positional(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DynamicCompletionCallSchema {
    pub call: JsonValue,
    pub strip: bool,
    pub pos: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PipelineDataHeaderSchema {
    Empty,
    Value(ValueSchema, Option<PipelineMetadataSchema>),
    ListStream(ListStreamInfoSchema),
    ByteStream(ByteStreamInfoSchema),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListStreamInfoSchema {
    pub id: usize,
    pub span: SpanSchema,
    pub metadata: Option<PipelineMetadataSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ByteStreamInfoSchema {
    pub id: usize,
    pub span: SpanSchema,
    #[serde(rename = "type")]
    pub type_: ByteStreamTypeSchema,
    pub metadata: Option<PipelineMetadataSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ByteStreamTypeSchema {
    Binary,
    String,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PipelineMetadataSchema {
    pub data_source: DataSourceSchema,
    pub path_columns: Vec<String>,
    pub content_type: Option<String>,
    #[serde(default)]
    pub custom: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DataSourceSchema {
    Ls,
    HtmlThemes,
    FilePath(String),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StreamDataSchema {
    List(ValueSchema),
    Raw(ResultBytesSchema),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ResultBytesSchema {
    Ok(Vec<u8>),
    Err(JsonValue),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StreamMessageSchema {
    Data(usize, StreamDataSchema),
    End(usize),
    Drop(usize),
    Ack(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PluginCallResponseSchema<D> {
    Ok,
    Error(JsonValue),
    Metadata(JsonValue),
    Signature(Vec<JsonValue>),
    Ordering(Option<OrderingSchema>),
    CompletionItems(Option<Vec<JsonValue>>),
    PipelineData(D),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum OrderingSchema {
    Less,
    Equal,
    Greater,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum EngineCallSchema<D> {
    GetConfig,
    GetPluginConfig,
    GetEnvVar(String),
    GetEnvVars,
    GetCurrentDir,
    AddEnvVar(String, ValueSchema),
    GetHelp,
    EnterForeground,
    LeaveForeground,
    GetSpanContents(SpanSchema),
    EvalClosure {
        closure: SpannedSchema<JsonValue>,
        positional: Vec<ValueSchema>,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
    FindDecl(String),
    GetBlockIR(usize),
    CallDecl {
        decl_id: usize,
        call: EvaluatedCallSchema,
        input: D,
        redirect_stdout: bool,
        redirect_stderr: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum EngineCallResponseSchema<D> {
    Error(JsonValue),
    PipelineData(D),
    Config(JsonValue),
    ValueMap(std::collections::HashMap<String, ValueSchema>),
    Identifier(usize),
    IrBlock(JsonValue),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum SignalActionSchema {
    Interrupt,
    Reset,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CustomValueOpSchema {
    ToBaseValue,
    FollowPathInt {
        index: SpannedSchema<usize>,
        optional: bool,
    },
    FollowPathString {
        column_name: SpannedSchema<String>,
        optional: bool,
        casing: CasingSchema,
    },
    PartialCmp(ValueSchema),
    Operation(SpannedSchema<JsonValue>, ValueSchema),
    Save {
        path: SpannedSchema<String>,
        save_call_span: SpanSchema,
    },
    Dropped,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum CasingSchema {
    Sensitive,
    Insensitive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PluginCustomValueSchema {
    pub name: String,
    pub data: Vec<u8>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub notify_on_drop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpanSchema {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpannedSchema<T> {
    pub item: T,
    pub span: SpanSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ValueSchema {
    Bool {
        val: bool,
        span: SpanSchema,
    },
    Int {
        val: i64,
        span: SpanSchema,
    },
    Float {
        val: f64,
        span: SpanSchema,
    },
    String {
        val: String,
        span: SpanSchema,
    },
    Glob {
        val: String,
        no_expand: bool,
        span: SpanSchema,
    },
    Filesize {
        val: JsonValue,
        span: SpanSchema,
    },
    Duration {
        val: i64,
        span: SpanSchema,
    },
    Date {
        val: String,
        span: SpanSchema,
    },
    Range {
        val: JsonValue,
        span: SpanSchema,
    },
    Record {
        val: JsonValue,
        span: SpanSchema,
    },
    List {
        vals: Vec<ValueSchema>,
        span: SpanSchema,
    },
    Closure {
        val: JsonValue,
        span: SpanSchema,
    },
    Error {
        error: JsonValue,
        span: SpanSchema,
    },
    Binary {
        val: Vec<u8>,
        span: SpanSchema,
    },
    CellPath {
        val: JsonValue,
        span: SpanSchema,
    },
    Custom {
        val: JsonValue,
        span: SpanSchema,
    },
    Nothing {
        span: SpanSchema,
    },
}

fn is_false(value: &bool) -> bool {
    !value
}

pub fn plugin_protocol_schema_json() -> Result<JsonValue, serde_json::Error> {
    serde_json::to_value(schema_for!(PluginProtocolSchemaDocument))
}

pub fn plugin_protocol_schema_pretty() -> Result<String, serde_json::Error> {
    let schema = plugin_protocol_schema_json()?;
    serde_json::to_string_pretty(&schema)
}
