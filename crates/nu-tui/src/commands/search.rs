use super::WidgetKind;
use super::{builder_io_types, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSearch;

impl Command for TuiSearch {
    fn name(&self) -> &str {
        "tui search"
    }

    fn description(&self) -> &str {
        "Add a search box that filters tables, logs, and trees as you type."
    }

    fn extra_description(&self) -> &str {
        "`--bind` focuses this search box from anywhere except a text field (for example `--bind /` or `--bind ctrl+r`). Esc clears the query; Esc again leaves the box.\n\
         \n\
         Scope comes from position: a search box in the outer pipeline filters every table, log, and tree; one placed inside `tui split [...]` or `tui tab \"name\" [...]` filters only the widgets in that container."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui search")
            .category(Category::Viewers)
            .named(
                "placeholder",
                SyntaxShape::String,
                "Placeholder text.",
                None,
            )
            .named(
                "bind",
                SyntaxShape::String,
                "Key that focuses search, for example '/' or 'ctrl+r'.",
                None,
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Filter a table from a search box",
            example: r#"[{name: alpha},{name: beta}] | tui search --placeholder "filter" --bind / | tui table | tui debug --keys "/,type:be,tab,enter" | get selected.name"#,
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
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "search",
                WidgetKind::Search { placeholder, bind },
                Vec::new(),
            )
        })
    }
}
