use super::{WidgetKind, builder_io_types, common_flags, flag_strings, push_widget, with_app};
use crate::widgets::search::SearchWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSearch;

impl Command for TuiSearch {
    fn name(&self) -> &str {
        "tui search"
    }

    fn description(&self) -> &str {
        "Add a search box that filters tables, logs, trees, and selects as you type."
    }

    fn extra_description(&self) -> &str {
        "`--bind` focuses this search box from anywhere except a text field (for example `--bind /` or `--bind ctrl+r`). Esc clears the query; Esc again leaves the box. Enter submits the highlighted row of the list it filters.\n\
         \n\
         Matching is a case-insensitive substring over every field by default. `--fuzzy` uses the same matcher as `input list --fuzzy`, `--case-sensitive` respects case, and `--columns [name]` matches only those columns.\n\
         \n\
         Scope comes from position: a search box in the outer pipeline filters every list; one placed inside `tui split [...]` or `tui box \"name\" [...]` filters only the widgets in that container."
    }

    fn signature(&self) -> Signature {
        common_flags(
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
                .switch("fuzzy", "Fuzzy matching instead of substring.", Some('f'))
                .switch("case-sensitive", "Respect case when matching.", Some('s'))
                .named(
                    "columns",
                    SyntaxShape::List(Box::new(SyntaxShape::String)),
                    "Match only these record columns.",
                    None,
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Filter a table from a search box",
                example: r#"[{name: alpha},{name: beta}] | tui search --placeholder "filter" --bind / | tui table | tui debug --keys "/,type:be,tab,enter" | get selected.name"#,
                result: None,
            },
            Example {
                description: "Fuzzy match on one column",
                example: r#"[{name: alpha},{name: gamma}] | tui search --fuzzy --columns [name] | tui table | tui debug --keys "tab,type:gm,enter" | get selected.name"#,
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
        with_app(call, input, |app| {
            let widget = SearchWidget {
                placeholder: call
                    .get_flag(engine_state, stack, "placeholder")?
                    .unwrap_or_else(|| "search".into()),
                bind: call.get_flag(engine_state, stack, "bind")?,
                fuzzy: call.has_flag(engine_state, stack, "fuzzy")?,
                case_sensitive: call.has_flag(engine_state, stack, "case-sensitive")?,
                columns: flag_strings(engine_state, stack, call, "columns")?,
            };
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Search(widget),
                Vec::new(),
                None,
            )
        })
    }
}
