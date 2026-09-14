//! `tui` parent command and each `tui *` subcommand.

mod body;
mod keybindings;
mod label;
mod list;
mod log;
mod menu;
mod preview;
mod run;
mod search;
mod splitter;
mod status;
mod tab;
mod table;
mod tabs;
mod textbox;
mod title;
mod tree;

use super::app::{TuiApp, tui_type};
use super::widget::{Place, Rel, Widget, WidgetKind};
use nu_engine::{command_prelude::*, get_full_help};
use nu_protocol::{PipelineData, Type};

pub use body::TuiBody;
pub use keybindings::TuiKeybindings;
pub use label::TuiLabel;
pub use list::TuiList;
pub use log::TuiLog;
pub use menu::TuiMenu;
pub use preview::TuiPreview;
pub use run::TuiRun;
pub use search::TuiSearch;
pub use splitter::TuiSplitter;
pub use status::TuiStatus;
pub use tab::TuiTab;
pub use table::TuiTable;
pub use tabs::TuiTabs;
pub use textbox::TuiTextBox;
pub use title::TuiTitle;
pub use tree::TuiTree;

pub(super) fn empty_tui() -> Type {
    tui_type()
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

pub(super) fn flag_strings(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    name: &str,
) -> Result<Vec<String>, ShellError> {
    let Some(val) = call.get_flag::<Value>(engine_state, stack, name)? else {
        return Ok(Vec::new());
    };
    match val {
        Value::List { vals, .. } => {
            let mut out = Vec::with_capacity(vals.len());
            for v in vals {
                match v.as_str() {
                    Ok(s) => out.push(s.to_string()),
                    Err(_) => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!("expected string, found {}", v.get_type()),
                            span: v.span(),
                        });
                    }
                }
            }
            Ok(out)
        }
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
    if app.widgets.is_empty() {
        if let Value::String { val, .. } = &app.data {
            let text = val.clone();
            app.data = Value::nothing(call.head);
            return Ok(text);
        }
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

pub(super) fn place_from_call(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<Place>, ShellError> {
    let right: Option<String> = call.get_flag(engine_state, stack, "right-of")?;
    let left: Option<String> = call.get_flag(engine_state, stack, "left-of")?;
    let above: Option<String> = call.get_flag(engine_state, stack, "above")?;
    let below: Option<String> = call.get_flag(engine_state, stack, "below")?;
    let ratio = call
        .get_flag::<i64>(engine_state, stack, "ratio")?
        .unwrap_or(50)
        .clamp(10, 90) as u16;

    let mut found = Vec::new();
    if let Some(of) = right {
        found.push((Rel::RightOf, of));
    }
    if let Some(of) = left {
        found.push((Rel::LeftOf, of));
    }
    if let Some(of) = above {
        found.push((Rel::Above, of));
    }
    if let Some(of) = below {
        found.push((Rel::Below, of));
    }
    match found.len() {
        0 => Ok(None),
        1 => {
            let (rel, of) = found.remove(0);
            Ok(Some(Place { rel, of, ratio }))
        }
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "use only one of --right-of, --left-of, --above, --below".into(),
            span: call.head,
        }),
    }
}

pub(super) fn placement_flags(sig: Signature) -> Signature {
    sig.named(
        "right-of",
        SyntaxShape::String,
        "Place this widget to the right of the given widget id.",
        None,
    )
    .named(
        "left-of",
        SyntaxShape::String,
        "Place this widget to the left of the given widget id.",
        None,
    )
    .named(
        "above",
        SyntaxShape::String,
        "Place this widget above the given widget id.",
        None,
    )
    .named(
        "below",
        SyntaxShape::String,
        "Place this widget below the given widget id.",
        None,
    )
    .named(
        "ratio",
        SyntaxShape::Int,
        "Percent of space for the anchor widget when using a placement flag (default 50).",
        None,
    )
}

pub(super) fn push_widget(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    app: &mut TuiApp,
    prefix: &str,
    kind: WidgetKind,
) -> Result<(), ShellError> {
    let requested: Option<String> = call.get_flag(engine_state, stack, "id")?;
    let id = app.next_id(prefix, requested, call.head)?;
    let place = place_from_call(engine_state, stack, call)?;
    app.push(Widget { id, kind, place });
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
         Pipeline data (including lazy streams) flows through `tui *` builders without being collected. `tui run` reads the stream while the UI is open, so rows appear as they are produced."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui")
            .category(Category::Viewers)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["ratatui", "interactive", "popup", "ui", "terminal"]
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
