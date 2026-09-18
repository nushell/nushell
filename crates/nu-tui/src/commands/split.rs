use super::{
    WidgetKind, builder_io_types, children_from_values, common_flags, push_widget, with_app,
};
use crate::widget::Size;
use crate::widgets::split::{SplitDir, SplitWidget, parse_size};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct TuiSplit;

impl Command for TuiSplit {
    fn name(&self) -> &str {
        "tui split"
    }

    fn description(&self) -> &str {
        "Lay out child widgets side by side (or stacked with --vertical) with draggable dividers."
    }

    fn extra_description(&self) -> &str {
        "Children are `tui` values built in parentheses: `tui split [(tui table) (tui preview)]`. A child can carry its own rows: `[(ls | tui table) (ps | tui table)]`. Splits nest.\n\
         \n\
         `--sizes` gives one size per child: an int is cells (`20`), `\"30%\"` a share of the split, `\"1fr\"` a share of what is left (`\"2fr\"` twice as much), `\"min:10\"` / `\"max:40\"` bounds. Missing entries are `1fr`. `--ratio 60` is shorthand for `--sizes [60% 1fr]`. Drag a divider with the mouse, or Tab to the split and use arrows/hjkl for 1% steps."
    }

    fn signature(&self) -> Signature {
        common_flags(
            Signature::build("tui split")
                .category(Category::Viewers)
                .required(
                    "children",
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    "Widgets to arrange, e.g. [(tui table) (tui preview)].",
                )
                .switch("vertical", "Stack children top to bottom.", Some('v'))
                .named(
                    "sizes",
                    SyntaxShape::List(Box::new(SyntaxShape::Any)),
                    "One size per child: cells, \"30%\", \"1fr\", \"min:N\", \"max:N\".",
                    Some('s'),
                )
                .named(
                    "ratio",
                    SyntaxShape::Int,
                    "Percent of space for the first child; the rest share the remainder.",
                    Some('r'),
                ),
        )
        .input_output_types(builder_io_types())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Table on the left, file preview on the right",
                example: "ls | tui split --sizes [40% 1fr] [(tui table --columns [name size]) (tui preview)] | tui run",
                result: None,
            },
            Example {
                description: "A fixed 30-cell sidebar",
                example: "ls | tui split --sizes [30 1fr] [(tui tree --walk) (tui preview)] | tui run",
                result: None,
            },
            Example {
                description: "Two tables with their own rows",
                example: "tui split [(ls | tui table) (ps | tui table)] | tui run",
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
        let children = children_from_values(call.req(engine_state, stack, 0)?)?;
        let direction = if call.has_flag(engine_state, stack, "vertical")? {
            SplitDir::Vertical
        } else {
            SplitDir::Horizontal
        };
        let mut sizes: Vec<Size> = match call.get_flag::<Value>(engine_state, stack, "sizes")? {
            Some(Value::List { vals, .. }) => {
                vals.iter().map(parse_size).collect::<Result<_, _>>()?
            }
            Some(other) => vec![parse_size(&other)?],
            None => Vec::new(),
        };
        if let Some(ratio) = call.get_flag::<i64>(engine_state, stack, "ratio")? {
            if !sizes.is_empty() {
                return Err(ShellError::IncompatibleParametersSingle {
                    msg: "use only one of --sizes, --ratio".into(),
                    span: call.head,
                });
            }
            sizes = vec![Size::Percent(ratio.clamp(5, 95) as u16), Size::Fill(1)];
        }
        with_app(call, input, |app| {
            push_widget(
                engine_state,
                stack,
                call,
                app,
                WidgetKind::Split(SplitWidget { direction, sizes }),
                children,
                None,
            )
        })
    }
}
