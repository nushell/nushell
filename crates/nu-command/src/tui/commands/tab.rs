use super::{WidgetKind, builder_io_types, children_from_values, push_widget, with_app};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTab;

impl Command for TuiTab {
    fn name(&self) -> &str {
        "tui tab"
    }

    fn description(&self) -> &str {
        "Group widgets under a name: a page in the tab bar, or a titled box when nested."
    }

    fn extra_description(&self) -> &str {
        "In the outer pipeline each `tui tab` is a page and the tab bar shows whenever any exists. Click a tab, press 1-9, or use `[` `]` / Ctrl+Tab to switch. Chrome (title, menu, search, status) stays visible on every page.\n\
         \n\
         Inside `tui split [...]` a tab is not a page: it draws a bordered box with the title around its children.\n\
         \n\
         Children are `tui` values built with no pipeline input, in parentheses: `tui tab \"files\" [(tui table) (tui preview)]`."
    }

    fn signature(&self) -> Signature {
        Signature::build("tui tab")
            .category(Category::Viewers)
            .required("title", SyntaxShape::String, "Tab title.")
            .required(
                "children",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Widgets on this tab, e.g. [(tui table) (tui preview)].",
            )
            .named("id", SyntaxShape::String, "Widget id.", None)
            .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "The same rows as a table and as a tree, on two tabs",
                example: r#"ls | tui tab "table" [(tui table)] | tui tab "tree" [(tui tree --walk)] | tui run"#,
                result: None,
            },
            Example {
                description: "A titled box inside a split",
                example: r#"ls | tui split [(tui tab "list" [(tui table)]) (tui preview)] | tui run"#,
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
        let title: String = call.req(engine_state, stack, 0)?;
        let children = children_from_values(call.req(engine_state, stack, 1)?)?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                "tab",
                WidgetKind::Tab { title },
                children,
            )
        })
    }
}
