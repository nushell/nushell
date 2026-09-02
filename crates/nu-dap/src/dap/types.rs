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
    /// Why a breakpoint could not be verified — shown by the client next to
    /// the greyed-out marker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArgs {
    /// Required by the spec, ignored here: one script runs on one eval
    /// thread, so there is nothing to select.
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
    /// `"hover"`, `"watch"` or `"repl"`. Accepted but not acted on: all three
    /// take the same path in `on_evaluate`.
    #[serde(default)]
    pub context: Option<String>,
}

// ---------------------------------------------------------------------------
// Message bodies
//
// One type per request we answer and per event we emit, so no handler
// hand-rolls a JSON object and nothing reaches the wire that isn't declared
// here.
//
// The two halves are shaped differently because the wire is: an event names
// itself in an `event` field, so [`DapEvent`] is one enum whose variant *is*
// that name. A response carries no such tag — the client matches it to the
// request by `request_seq` — so there is nothing for the type to declare, and
// each body is a plain struct marked with [`ResponseBody`].
// ---------------------------------------------------------------------------

/// Body of a successful response. A marker trait: the command is dictated by
/// the request being answered, so the body only has to be a declared type.
pub trait ResponseBody: Serialize {}

/// Requests that are acknowledged with no body (`nuDapUiReply`, `stepBack`).
impl ResponseBody for () {}

/// `initialize` response: what this adapter supports. Anything omitted is
/// false by default, so only the flags we actually honour are listed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub supports_configuration_done_request: bool,
    pub supports_terminate_request: bool,
    /// Hot restart: re-run the (possibly edited) script from disk in the same
    /// session, keeping breakpoints.
    pub supports_restart_request: bool,
    pub supports_conditional_breakpoints: bool,
    pub supports_log_points: bool,
    pub supports_exception_info_request: bool,
    /// Time travel: VS Code shows Step Back / Reverse Continue buttons and
    /// sends `stepBack`/`reverseContinue`.
    pub supports_step_back: bool,
    pub supports_evaluate_for_hovers: bool,
    /// Explicitly unsupported for v1.
    pub supports_function_breakpoints: bool,
    pub exception_breakpoint_filters: Vec<ExceptionBreakpointFilter>,
}

/// One checkbox in the client's Breakpoints panel.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointFilter {
    /// Echoed back in `setExceptionBreakpoints.filters`.
    pub filter: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub default: bool,
}

/// `setBreakpoints` / `setExceptionBreakpoints` response.
#[derive(Debug, Serialize)]
pub struct SetBreakpointsResponse {
    pub breakpoints: Vec<Breakpoint>,
}

/// `exceptionInfo` response, served from the recorded stop.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionInfoResponse {
    pub exception_id: String,
    pub description: String,
    /// Always `"always"`: we only ever stop on errors the user opted into.
    pub break_mode: &'static str,
}

/// `threads` response. Nushell evaluation is single-threaded from the
/// client's point of view, so this is always one entry.
#[derive(Debug, Serialize)]
pub struct ThreadsResponse {
    pub threads: Vec<Thread>,
}

#[derive(Debug, Serialize)]
pub struct Thread {
    pub id: i64,
    pub name: &'static str,
}

/// `stackTrace` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceResponse {
    pub stack_frames: Vec<StackFrame>,
    pub total_frames: usize,
}

/// `scopes` response.
#[derive(Debug, Serialize)]
pub struct ScopesResponse {
    pub scopes: Vec<Scope>,
}

/// `variables` response.
#[derive(Debug, Serialize)]
pub struct VariablesResponse {
    pub variables: Vec<Variable>,
}

/// `evaluate` response (hover, watch and repl all take this path).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResponse {
    pub result: String,
    /// 0 = leaf; >0 = expandable in the client.
    pub variables_reference: i64,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

/// `continue` / `reverseContinue` response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponse {
    pub all_threads_continued: bool,
}

/// Body of the custom `nuDapVisualize` response: the full value as JSON,
/// unlike the bounded Variables tree.
#[derive(Debug, Serialize)]
pub struct VisualizeResponse {
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub type_: String,
    /// Set when the value hit the preview limits and was clipped.
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Every event this adapter emits, with its body.
///
/// Adjacent tagging is exactly the DAP event envelope: the variant name
/// serializes into `event` and its fields into `body`, so the name cannot
/// drift from the shape. `rename_all` maps the variant to its wire spelling
/// (`Stopped` → `"stopped"`, `NuDapIr` → `"nuDapIr"`), and a unit variant
/// omits `body` entirely — which is what a bodyless event should look like.
#[derive(Debug, Serialize)]
#[serde(tag = "event", content = "body", rename_all = "camelCase")]
pub enum DapEvent {
    /// Configuration (breakpoints, exception filters) may now be sent.
    Initialized,

    /// The debuggee has come to a halt; the client should refresh its
    /// stack/scopes/variables views.
    #[serde(rename_all = "camelCase")]
    Stopped {
        /// A protocol-defined reason (`"breakpoint"`, `"step"`, `"exception"`,
        /// …) — the client keys UI off it, so it is never free text.
        reason: &'static str,
        thread_id: i64,
        all_threads_stopped: bool,
        /// Shown in the callstack view next to the stopped frame.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Same text as `description`, repeated because VS Code renders this
        /// one in the notification popup and the other in the callstack.
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },

    /// The session is over. Bodyless — a `restart` field would ask the client
    /// to relaunch, which we never do.
    Terminated,

    /// The script finished (or panicked, reported as a failure).
    #[serde(rename_all = "camelCase")]
    Exited { exit_code: i64 },

    /// Everything the debuggee wrote, tagged with the stream it came from
    /// (`"stdout"` / `"stderr"`).
    Output { category: String, output: String },

    /// A breakpoint's verified state or line changed after parsing resolved
    /// which lines are actually steppable.
    Breakpoint {
        /// `"changed"`, `"new"` or `"removed"`; we only ever send `"changed"`.
        reason: &'static str,
        breakpoint: Breakpoint,
    },

    /// Custom: IR listing for the extension's "Show IR" panel. Clients that
    /// don't know this event simply ignore it.
    #[serde(rename_all = "camelCase")]
    NuDapIr {
        text: String,
        instruction_index: usize,
        instruction_count: usize,
    },

    /// Custom: an `input` / `input list` prompt for the extension to show,
    /// answered by a `nuDapUiReply` request carrying the same `id`. This is
    /// the whole wire contract for the UI bridge — `kind` picks the shape and
    /// the fields it doesn't use are omitted.
    NuDapUi {
        /// Correlates the reply; the eval thread blocks on it.
        id: u64,
        /// `"text"` for `input`, `"list"` for `input list`.
        kind: &'static str,
        prompt: String,
        /// `"text"` only: prefilled value.
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        /// `"list"` only: the choices, already rendered to labels.
        #[serde(skip_serializing_if = "Option::is_none")]
        items: Option<Vec<String>>,
        /// `"list"` only: allow multiple selection.
        #[serde(skip_serializing_if = "Option::is_none")]
        multi: Option<bool>,
        /// `"list"` only: the choices were clipped to a display cap.
        #[serde(skip_serializing_if = "Option::is_none")]
        truncated: Option<bool>,
    },
}

impl ResponseBody for Capabilities {}
impl ResponseBody for SetBreakpointsResponse {}
impl ResponseBody for ExceptionInfoResponse {}
impl ResponseBody for ThreadsResponse {}
impl ResponseBody for StackTraceResponse {}
impl ResponseBody for ScopesResponse {}
impl ResponseBody for VariablesResponse {}
impl ResponseBody for EvaluateResponse {}
impl ResponseBody for ContinueResponse {}
impl ResponseBody for VisualizeResponse {}
