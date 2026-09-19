use super::{WidgetKind, builder_io_types, push_widget, selectable_flags, with_app};
use crate::widgets::select::{Display, SelectWidget};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSelect;

impl Command for TuiSelect {
    fn name(&self) -> &str {
        "tui select"
    }

    fn description(&self) -> &str {
        "Add a radio list of items, or a checkbox list with --multi."
    }

    fn extra_description(&self) -> &str {
        "Items come from the list argument, from `--data`, or from the pipeline. Up/Down move, Space checks (with `--multi`), Enter submits the chosen item(s); `--index` submits positions instead.\n\
         \n\
         `--display` picks the text shown for each item: a column name for records, or a closure over the item. The original item is what gets selected, as with `input list --display`."
    }

    fn signature(&self) -> Signature {
        selectable_flags(
            Signature::build("tui select")
                .category(Category::Viewers)
                .optional(
                    "items",
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    "Items to choose from. Defaults to the widget's data rows.",
                )
                .named(
                    "display",
                    SyntaxShape::OneOf(vec![
                        SyntaxShape::String,
                        SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                    ]),
                    "Column or closure that produces each item's label.",
                    Some('d'),
                )
                .switch("index", "Select item indexes instead of items.", Some('i')),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Pick one of three options",
                example: "tui select [small medium large] | tui debug --keys [down enter] | get selected",
                result: None,
            },
            Example {
                description: "Check several files by name",
                example: "ls | tui select --multi --display name | tui run | get selected.name",
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
        let items: Vec<Value> = call.opt(engine_state, stack, 0)?.unwrap_or_default();
        let display = match call.get_flag::<Value>(engine_state, stack, "display")? {
            None => None,
            Some(Value::String { val, .. }) => Some(Display::Column(val)),
            Some(Value::Closure { val, .. }) => Some(Display::Closure(*val)),
            Some(other) => {
                return Err(ShellError::TypeMismatch {
                    err_message: format!(
                        "expected a column name or closure, found {}",
                        other.get_type()
                    ),
                    span: other.span(),
                });
            }
        };
        with_app(call, input, |app| {
            let widget = SelectWidget {
                items,
                multi: call.has_flag(engine_state, stack, "multi")?,
                index: call.has_flag(engine_state, stack, "index")?,
                display,
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Select(widget),
                Vec::new(),
                None,
            )
        })
    }
}
