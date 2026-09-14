use super::WidgetKind;
use super::{empty_tui, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSearch;

impl Command for TuiSearch {
    fn name(&self) -> &str {
        "tui search"
    }

    fn description(&self) -> &str {
        "Add a search box that filters tables, lists, logs, trees, and keybinding lists as you type."
    }

    fn extra_description(&self) -> &str {
        "`--bind` focuses this search box (for example `--bind /` or `--bind ctrl+r`). `--target` limits filtering to that widget id. Untargeted search boxes filter every table, list, log, tree, and keybindings widget. There is one query string for the session."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui search")
            .category(Category::Viewers)
            .named("placeholder", SyntaxShape::String, "Placeholder text.", None)
            .named(
                "bind",
                SyntaxShape::String,
                "Key that focuses search, for example '/' or 'ctrl+r'.",
                None,
            )
            .named(
                "target",
                SyntaxShape::String,
                "Widget id to filter. Default: every table, list, log, tree, and keybindings widget.",
                None,
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(vec![
                (Type::Nothing, empty_tui()),
                (empty_tui(), empty_tui()),
                (Type::Any, empty_tui()),
            ])
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Filter a table from a search box",
            example: r#"[{name: alpha},{name: beta}] | tui search --placeholder "filter" --bind / | tui table | tui run --keys "/,type:be,tab,enter""#,
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        with_app(call, input, |app| {
            let placeholder = call
                .get_flag(engine_state, stack, "placeholder")?
                .unwrap_or_else(|| "search".into());
            let bind = call.get_flag(engine_state, stack, "bind")?;
            let target = call.get_flag(engine_state, stack, "target")?;
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "search",
                WidgetKind::Search {
                    placeholder,
                    bind,
                    target,
                },
            )
        })
    }
}
