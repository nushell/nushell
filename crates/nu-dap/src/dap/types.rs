//! Typed arguments/bodies for the subset of DAP we implement.
//! Only fields we actually read are declared; serde ignores the rest.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchArgs {
    /// Absolute path to the .nu script to debug.
    pub program: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub stop_on_entry: bool,
    /// Record every executed line for time-travel (Step Back / Reverse
    /// Continue). Default true. When false, only pause points are recorded.
    #[serde(default)]
    pub time_travel: Option<bool>,
    /// Ring-buffer cap for recorded steps. Default 10000.
    #[serde(default)]
    pub time_travel_max_steps: Option<usize>,
    /// Command to call as the entry point after top-level eval, for scripts
    /// that are function libraries with no `main`. Chosen via a dialog in the
    /// extension. When unset, falls back to `main` if defined.
    #[serde(default)]
    pub entry_point: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArgs {
    pub source: Source,
    #[serde(default)]
    pub breakpoints: Vec<SourceBreakpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: i64,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub log_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub verified: bool,
    pub line: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArgs {
    #[allow(dead_code)]
    pub thread_id: i64,
    #[serde(default)]
    pub start_frame: Option<i64>,
    #[serde(default)]
    pub levels: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    pub line: i64,
    pub column: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArgs {
    pub frame_id: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArgs {
    pub variables_reference: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// 0 = leaf; >0 = expandable, client will send a `variables` request with this id.
    pub variables_reference: i64,
}

/// Arguments of the custom `nuDapVisualize` request. Either address an
/// expandable node directly by its variablesReference, or a leaf (string,
/// binary, …) by its container's reference plus the variable name.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualizeArgs {
    #[serde(default)]
    pub variables_reference: Option<i64>,
    #[serde(default)]
    pub container_reference: Option<i64>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArgs {
    pub expression: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub context: Option<String>,
}
