use super::{
    WidgetKind, builder_io_types, children_from_values, common_flags, push_widget, with_app,
};
use crate::widgets::r#box::BoxWidget;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiBox;

impl Command for TuiBox {
    fn name(&self) -> &str {
        "tui box"
    }

    fn description(&self) -> &str {
        "Group widgets inside a titled, bordered box."
    }

    fn extra_description(&self) -> &str {
        "Children stack vertically inside the border. Boxes nest and can sit inside `tui split`. Children are `tui` values built in parentheses: `tui box \"files\" [(tui search) (tui table)]`."
    }

    fn signature(&self) -> Signature {
        common_flags(
            Signature::build("tui box")
                .category(Category::Viewers)
                .required("title", SyntaxShape::String, "Box title.")
                .required(
                    "children",
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    "Widgets inside the box, e.g. [(tui search) (tui table)].",
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "A titled box inside a split",
            example: r#"ls | tui split [(tui box "list" [(tui table)]) (tui preview)] | tui run"#,
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
                WidgetKind::Box(BoxWidget { title }),
                children,
                None,
            )
        })
    }
}
