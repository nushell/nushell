use plotly::common::Mode;
use plotly::{Layout, Plot, Scatter};

use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ScatterPlot;

impl Command for ScatterPlot {
    fn name(&self) -> &str {
        "dfr scatter"
    }

    fn usage(&self) -> &str {
        "Create and display a scatter plot from dataframe columns."
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
            .required(
                "xvar",
                SyntaxShape::String,
                "variable to plot on the abscissa",
            )
            .required(
                "yvar",
                SyntaxShape::String,
                "variable to plot on the ordinate",
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
) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head);
    let lazy = NuLazyFrame::try_from_value(value)?;

    let lazy = lazy.into_polars();

    let x: Value = call.req(engine_state, stack, 0)?;
    let y: Value = call.req(engine_state, stack, 1)?;

    let x = NuExpression::extract_exprs(x)?;
    let y = NuExpression::extract_exprs(y)?;

    let x_vec = lazy.clone().select(x).collect().unwrap();
    let y_vec = lazy.select(y).collect().unwrap();

    let trace1 = Scatter::new(x_vec.into(), y_vec.into())
        .name("trace1")
        .mode(Mode::Markers);

    let mut plot = Plot::new();
    plot.add_trace(trace1);

    let layout = Layout::new().title("<b>Scatter Plot</b>".into());
    plot.set_layout(layout);

    plot.show();

    Ok(PipelineData::Value(
        Value::Nothing {
            internal_span: call.head,
        },
        None,
    ))
}
#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ScatterPlot {})])
    }
}
