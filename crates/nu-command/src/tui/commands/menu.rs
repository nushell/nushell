use super::{WidgetKind, builder_io_types, push_widget, with_app};
use crate::tui::widget::MenuItem;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiMenu;

impl Command for TuiMenu {
    fn name(&self) -> &str {
        "tui menu"
    }

    fn description(&self) -> &str {
        "Add a menu bar with mnemonics, dropdown items, and actions."
    }

    fn extra_description(&self) -> &str {
        "Each entry is a string, or a record `{name, items?, action?}`. `items` is a list of the same shape and opens as a dropdown; `action` is a closure.\n\
         \n\
         Mnemonics: `&` before a letter in a name picks it (`\"&File\"` shows as File with F underlined); otherwise the first letter is used. Alt+letter opens that bar item from anywhere except a text field. Inside an open dropdown, the letter alone picks the item. Left/Right (or h/l) move along the bar, Down/Enter open a dropdown, Up/Down move inside it, Esc closes it.\n\
         \n\
         Activating an item with an `action` runs the closure: a returned value replaces the data list (every table, log, and tree redraws); `nothing` leaves the data alone; an error goes to the status bar. Activating an item without an action submits it: `selected` is `{menu: \"File\", item: \"Open\", row: <highlighted table row>}` for a dropdown entry, or the bar item's name.\n\
         \n\
         The menu is top-level chrome and cannot be nested in `tui split` or `tui tab`."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui menu")
            .category(Category::Viewers)
            .required(
                "items",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Bar entries: strings or {name, items?, action?} records.",
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "A plain bar; Enter submits the item name",
                example: r#"tui label --title "Editor" | tui menu ["&File" "&Edit" "&View"] | tui table | tui debug --keys "alt+e,enter" | get selected"#,
                result: None,
            },
            Example {
                description: "Dropdowns with actions that reload the table, and a Quit entry that submits",
                example: r#"ls | tui menu [{name: "&File", items: [{name: "&Reload", action: {|| ls }} "&Quit"]} {name: "&View", items: [{name: "&Sizes", action: {|| ls | sort-by size }}]}] | tui table | tui run"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let items = menu_items(call.req(engine_state, stack, 0)?)?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "menu",
                WidgetKind::Menu { items },
                Vec::new(),
            )
        })
    }
}

/// `[name | {name, items?, action?}]` → menu items, recursively.
fn menu_items(value: Value) -> Result<Vec<MenuItem>, ShellError> {
    let span = value.span();
    let entries = match value {
        Value::List { vals, .. } => vals,
        other => {
            return Err(ShellError::TypeMismatch {
                err_message: format!("expected a list of menu items, found {}", other.get_type()),
                span: other.span(),
            });
        }
    };
    entries
        .into_iter()
        .map(|entry| match entry {
            Value::String { val, .. } => Ok(MenuItem::new(&val, Vec::new(), None)),
            Value::Record { val, .. } => {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str().ok())
                    .ok_or_else(|| ShellError::MissingParameter {
                        param_name: "name".into(),
                        span,
                    })?;
                let items = match val.get("items") {
                    Some(items) => menu_items(items.clone())?,
                    None => Vec::new(),
                };
                let action = match val.get("action") {
                    None => None,
                    Some(Value::Closure { val, .. }) => Some(*val.clone()),
                    Some(other) => {
                        return Err(ShellError::TypeMismatch {
                            err_message: format!(
                                "menu action must be a closure, found {}",
                                other.get_type()
                            ),
                            span: other.span(),
                        });
                    }
                };
                Ok(MenuItem::new(name, items, action))
            }
            other => Err(ShellError::TypeMismatch {
                err_message: format!(
                    "menu items are strings or {{name, items, action}} records, found {}",
                    other.get_type()
                ),
                span: other.span(),
            }),
        })
        .collect()
}
