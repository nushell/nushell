use super::{
    WidgetKind, builder_io_types, children_from_values, common_flags, push_widget, with_app,
};
use crate::widgets::tab::TabWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiTab;

impl Command for TuiTab {
    fn name(&self) -> &str {
        "tui tab"
    }

    fn description(&self) -> &str {
        "Group widgets as a page in the tab bar."
    }

    fn extra_description(&self) -> &str {
        "Each `tui tab` in the outer pipeline is a page and the tab bar shows whenever any exists. Click a tab, press 1-9, or use `[` `]` / Ctrl+Tab to switch. Chrome (title, menu, search, status) stays visible on every page.\n\
         \n\
         Tabs cannot be nested; for a titled box inside a split use `tui box`.\n\
         \n\
         Children are `tui` values built in parentheses: `tui tab \"files\" [(tui table) (tui preview)]`."
    }

    fn signature(&self) -> Signature {
        common_flags(
            Signature::build("tui tab")
                .category(Category::Viewers)
                .required("title", SyntaxShape::String, "Tab title.")
                .required(
                    "children",
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    "Widgets on this tab, e.g. [(tui table) (tui preview)].",
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "The same rows as a table and as a tree, on two tabs",
            example: r#"ls | tui tab "table" [(tui table)] | tui tab "tree" [(tui tree --walk)] | tui run"#,
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
        let title: String = call.req(engine_state, stack, 0)?;
        let children = children_from_values(call.req(engine_state, stack, 1)?)?;
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Tab(TabWidget { title }),
                children,
                None,
            )
        })
    }
}
