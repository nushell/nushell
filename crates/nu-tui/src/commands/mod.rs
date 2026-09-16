//! `tui` parent command and each `tui *` subcommand.

mod debug;
mod label;
mod log;
mod menu;
mod preview;
mod run;
mod search;
mod split;
mod tab;
mod table;
mod textbox;
mod tree;

use super::app::{TuiApp, tui_type};
use super::widget::{Widget, WidgetKind};
use nu_engine::{command_prelude::*, get_full_help};
use nu_protocol::{PipelineData, Type};

pub use debug::TuiDebug;
pub use label::TuiLabel;
pub use log::TuiLog;
pub use menu::TuiMenu;
pub use preview::TuiPreview;
pub use run::TuiRun;
pub use search::TuiSearch;
pub use split::TuiSplit;
pub use tab::TuiTab;
pub use table::TuiTable;
pub use textbox::TuiTextBox;
pub use tree::TuiTree;

pub(super) fn empty_tui() -> Type {
    tui_type()
}

/// Input/output types shared by every builder: start a TUI from nothing,
/// extend one, or attach to pipeline data that keeps flowing.
pub(super) fn builder_io_types() -> Vec<(Type, Type)> {
    vec![
        (Type::Nothing, empty_tui()),
        (empty_tui(), empty_tui()),
        (Type::Any, empty_tui()),
    ]
}

pub(super) fn with_app(
    call: &Call,
    input: PipelineData,
    f: impl FnOnce(&mut TuiApp) -> Result<(), ShellError>,
) -> Result<PipelineData, ShellError> {
    let (mut app, data) = TuiApp::split_input(input, call.head)?;
    f(&mut app)?;
    Ok(app.emit(data, call.head))
}

/// A list of strings, or a single string, from a value.
pub(super) fn strings_from_value(val: Value) -> Result<Vec<String>, ShellError> {
    match val {
        Value::List { vals, .. } => vals
            .into_iter()
            .map(|v| match v {
                Value::String { val, .. } => Ok(val),
                other => Err(ShellError::TypeMismatch {
                    err_message: format!("expected string, found {}", other.get_type()),
                    span: other.span(),
                }),
            })
            .collect(),
        Value::String { val, .. } => Ok(vec![val]),
        other => Err(ShellError::TypeMismatch {
            err_message: format!(
                "expected string or list of strings, found {}",
                other.get_type()
            ),
            span: other.span(),
        }),
    }
}

pub(super) fn flag_strings(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    name: &str,
) -> Result<Vec<String>, ShellError> {
    match call.get_flag::<Value>(engine_state, stack, name)? {
        Some(val) => strings_from_value(val),
        None => Ok(Vec::new()),
    }
}

/// Positional text, or a scalar string piped in as the first widget's input.
pub(super) fn consume_text_arg(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    app: &mut TuiApp,
    required: bool,
    what: &str,
) -> Result<String, ShellError> {
    if let Some(text) = call.opt::<String>(engine_state, stack, 0)? {
        return Ok(text);
    }
    if app.widgets.is_empty()
        && let Value::String { val, .. } = &app.data
    {
        let text = val.clone();
        app.data = Value::nothing(call.head);
        return Ok(text);
    }
    if required {
        Err(ShellError::MissingParameter {
            param_name: what.into(),
            span: call.head,
        })
    } else {
        Ok(String::new())
    }
}

/// The children list of `tui split` / `tui tab`: `tui` values built with no
/// pipeline input, e.g. `[(tui table) (tui preview)]`. Validated here;
/// merged into the tree by [`TuiApp::adopt_children`].
pub(super) fn children_from_values(values: Vec<Value>) -> Result<Vec<TuiApp>, ShellError> {
    let mut out = Vec::new();
    for value in values {
        let Some(child) = TuiApp::from_value(&value) else {
            return Err(ShellError::TypeMismatch {
                err_message: format!(
                    "expected a tui value, found {}. Build children with no pipeline input, \
                     inside parentheses: [(tui table) (tui preview)]",
                    value.get_type()
                ),
                span: value.span(),
            });
        };
        if !child.data.is_nothing() {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "children must not carry data; pipe data into the outer pipeline instead"
                    .into(),
                span: value.span(),
            });
        }
        if let Some(chrome) = child
            .widgets
            .iter()
            .find(|w| w.kind.is_chrome() && !matches!(w.kind, WidgetKind::Search { .. }))
        {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: format!(
                    "`{}` is top-level chrome and cannot be nested; add it to the outer pipeline",
                    chrome.kind.type_name()
                ),
                span: value.span(),
            });
        }
        out.push(child.clone());
    }
    Ok(out)
}

/// `--size [width height]` as a pair of ints.
pub(super) fn size_flag(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<(i64, i64)>, ShellError> {
    let Some(value) = call.get_flag::<Value>(engine_state, stack, "size")? else {
        return Ok(None);
    };
    let span = value.span();
    let bad = || ShellError::TypeMismatch {
        err_message: "expected [width height], two ints".into(),
        span,
    };
    let vals = value.as_list().map_err(|_| bad())?;
    match vals {
        [w, h] => Ok(Some((
            w.as_int().map_err(|_| bad())?,
            h.as_int().map_err(|_| bad())?,
        ))),
        _ => Err(bad()),
    }
}

/// Current directory for resolving relative paths in previews and tree walks.
pub(super) fn session_cwd(engine_state: &EngineState, stack: &Stack) -> std::path::PathBuf {
    engine_state
        .cwd_as_string(Some(stack))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

pub(super) fn push_widget(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    app: &mut TuiApp,
    prefix: &str,
    kind: WidgetKind,
    children: Vec<TuiApp>,
) -> Result<(), ShellError> {
    let requested: Option<String> = call.get_flag(engine_state, stack, "id")?;
    let (id, auto_id) = app.next_id(prefix, requested, call.head)?;
    let children = app.adopt_children(children, &id, call.head)?;
    app.push(Widget {
        id,
        auto_id,
        kind,
        children,
    });
    Ok(())
}

#[derive(Clone)]
pub struct Tui;

impl Command for Tui {
    fn name(&self) -> &str {
        "tui"
    }

    fn description(&self) -> &str {
        "Build interactive terminal UIs by composing `tui` subcommands."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message.\n\
         \n\
         Pipeline data (including lazy streams) flows through `tui *` builders without being collected. `tui run` reads the stream while the UI is open, so rows appear as they are produced.\n\
         \n\
         Layout is nested: `tui split [(tui table) (tui preview)]` puts two widgets side by side, and `tui tab \"name\" [...]` makes a page. Children are built without pipeline input; data enters once through the outer pipeline."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui")
            .category(Category::Viewers)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ratatui", "interactive", "popup", "terminal"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "List TUI subcommands",
            example: "tui",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(
            get_full_help(self, engine_state, stack, call.head),
            call.head,
        )
        .into_pipeline_data())
    }
}
