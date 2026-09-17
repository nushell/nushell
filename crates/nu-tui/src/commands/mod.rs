//! `tui` parent command and each `tui *` subcommand.

mod bind;
mod r#box;
mod button;
mod debug;
mod label;
mod log;
mod menu;
mod preview;
mod progress;
mod run;
mod search;
mod select;
mod split;
mod tab;
mod table;
mod textbox;
mod tree;

use crate::app::{TuiApp, tui_type};
use crate::widget::{Source, Widget, WidgetKind};
use nu_engine::{command_prelude::*, get_full_help};
use nu_protocol::engine::Closure;
use nu_protocol::{PipelineData, Type};

pub use bind::TuiBind;
pub use r#box::TuiBox;
pub use button::TuiButton;
pub use debug::TuiDebug;
pub use label::TuiLabel;
pub use log::TuiLog;
pub use menu::TuiMenu;
pub use preview::TuiPreview;
pub use progress::TuiProgress;
pub use run::TuiRun;
pub use search::TuiSearch;
pub use select::TuiSelect;
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

/// Flags every builder takes.
pub(super) fn common_flags(sig: Signature) -> Signature {
    sig.named("id", SyntaxShape::String, "Widget id.", None)
        .switch("focus", "Start with this widget focused.", None)
}

/// Flags for widgets that show data and can follow another widget.
pub(super) fn data_flags(sig: Signature) -> Signature {
    common_flags(sig)
        .named(
            "data",
            SyntaxShape::Any,
            "Data for this widget alone, instead of the outer pipeline's.",
            None,
        )
        .named(
            "from",
            SyntaxShape::String,
            "Id of the table, tree, or select whose highlighted row drives this widget.",
            None,
        )
}

/// Flags for widgets with a highlighted row.
pub(super) fn selectable_flags(sig: Signature) -> Signature {
    data_flags(sig)
        .named(
            "on-select",
            SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
            "Hook run when the highlighted row changes; receives the state record.",
            None,
        )
        .switch(
            "multi",
            "Space toggles rows; the selection is the checked rows.",
            Some('m'),
        )
}

pub(super) fn with_app(
    call: &Call,
    input: PipelineData,
    f: impl FnOnce(&mut TuiApp) -> Result<(), ShellError>,
) -> Result<PipelineData, ShellError> {
    let (mut app, data) = TuiApp::split_input(input)?;
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

/// The children list of `tui split` / `tui box` / `tui tab`: `tui` values
/// built in parentheses, e.g. `[(tui table) (tui preview)]`. A child may
/// carry its own data (`(ls | tui table)`); chrome and pages may not nest.
pub(super) fn children_from_values(values: Vec<Value>) -> Result<Vec<TuiApp>, ShellError> {
    let mut out = Vec::new();
    for value in values {
        let Some(child) = TuiApp::from_value(&value) else {
            return Err(ShellError::TypeMismatch {
                err_message: format!(
                    "expected a tui value, found {}. Build children inside parentheses: \
                     [(tui table) (tui preview)]. To give a child its own rows, pipe a \
                     collected value into it or pass --data",
                    value.get_type()
                ),
                span: value.span(),
            });
        };
        if let Some(bad) = child.iter().find(|w| {
            (w.kind.is_chrome() && !matches!(w.kind, WidgetKind::Search(_)))
                || matches!(w.kind, WidgetKind::Tab(_))
        }) {
            let what = if matches!(bad.kind, WidgetKind::Tab(_)) {
                "`tui tab` is a page and cannot be nested; use `tui box` for a titled group"
                    .to_string()
            } else {
                format!(
                    "`{}` is top-level chrome and cannot be nested; add it to the outer pipeline",
                    bad.kind.type_name()
                )
            };
            return Err(ShellError::IncompatibleParametersSingle {
                msg: what,
                span: value.span(),
            });
        }
        if !child.binds.is_empty() {
            return Err(ShellError::IncompatibleParametersSingle {
                msg: "`tui bind` applies to the whole TUI; add it to the outer pipeline".into(),
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

/// Append a widget, reading the flags common to builders: `--id`, and for
/// data widgets `--data`, `--from`, `--on-select`. `source_closure` is the
/// positional closure that turns the source row into this widget's data.
pub(super) fn push_widget(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    app: &mut TuiApp,
    kind: WidgetKind,
    children: Vec<TuiApp>,
    source_closure: Option<Closure>,
) -> Result<(), ShellError> {
    let requested: Option<String> = call.get_flag(engine_state, stack, "id")?;
    let (id, auto_id) = app.next_id(kind.type_name(), requested, call.head)?;
    let children = app.adopt_children(children, &id, call.head)?;
    let from: Option<String> = call.get_flag(engine_state, stack, "from")?;
    let source = (from.is_some() || source_closure.is_some()).then_some(Source {
        from,
        closure: source_closure,
    });
    app.push(Widget {
        id,
        auto_id,
        kind,
        children,
        data: call.get_flag(engine_state, stack, "data")?,
        source,
        on_select: call.get_flag(engine_state, stack, "on-select")?,
        focus: call.has_flag(engine_state, stack, "focus")?,
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
         Builders (`tui table`, `tui split`, ...) append widgets to a `tui` value. `tui run` shows it and returns one record: `{action, focused, selected, page, values, rows, live}`, where `values` holds every widget's state by id. `tui debug` returns the same record plus the painted `screen` and the resolved layout, for scripts and tests.\n\
         \n\
         Data: a value piped into a builder is the shared data list. Lists and streams are collected in full (up to 100k rows); an external command's output (`tail -f log | tui log`) and an unbounded range (`1..`) stay live, and their rows appear as they are produced. When the TUI closes while an external command is still running, it is stopped. A widget can have its own rows with `--data`, or by piping into it inside a container's child list: `tui split [(ls | tui table) (ps | tui table)]`. `--from <id>` (with an optional closure) makes a widget follow another's highlighted row.\n\
         \n\
         Hooks: `tui bind`, menu actions, `tui button`, `--on-select`, and the `tui run` refresh closure all receive the state record and may return nothing, a new data list, or `{action: submit|quit, selected: ...}`.\n\
         \n\
         Layout: `tui split --sizes [30% 1fr]` arranges children; `tui box` groups them with a border; `tui tab` makes a page. Colors come from `$env.config.tui`."
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
