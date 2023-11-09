use plotly::common::Mode;
use plotly::{Layout, Plot, Scatter};

use crate::dataframe::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ScatterPlot;

impl Command for ScatterPlot {
    fn name(&self) -> &str {
        "dfr scatter"
    }

    fn usage(&self) -> &str {
        "Create and save a scatter plot from dataframe columns."
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
            .named(
                "file",
                SyntaxShape::Filepath,
                "file path to save plot",
                Some('f'),
            )
            .named(
                "xvar",
                SyntaxShape::String,
                "variable to plot on the abscissa",
                Some('x'),
            )
            .named(
                "yvar",
                SyntaxShape::String,
                "variable to plot on the ordinate",
                Some('y'),
            )
    }
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Appends a dataframe as new columns",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr scatter a b",
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<_, ShellError> {
    let file_name: Spanned<PathBuf> = call.get_flag(engine_state, stack, "file")?;

    let mut df = NuDataFrame::try_from_pipeline(input, call.head)?;

    pdf = df.into_polars();

    let x: Option<Spanned<String>> = call.get_flag(engine_state, stack, "xvar")?;
    let y: Option<Spanned<String>> = call.get_flag(engine_state, stack, "yvar")?;

    let trace1 = Scatter::new(
        pdf.select(x).into_iter().collect(),
        pdf.select(y).into_iter().collect(),
    )
    .name("trace1")
    .mode(Mode::Markers);

    let mut plot = Plot::new();
    plot.add_trace(trace1);

    let layout = Layout::new().title("<b>Scatter Plot</b>".into());
    plot.set_layout(layout);

    plot.show();

    Ok(())
}
