use super::rose_::create_plot;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct RosePlot;

impl Command for RosePlot {
    fn name(&self) -> &str {
        "plot rose"
    }

    fn usage(&self) -> &str {
        "Create and display a rose plot from pipeline data."
    }
    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Any, Type::Any)
            .category(Category::Custom("plotting".into()))
            .required(
                "labels",
                SyntaxShape::String,
                "category field for the rose plot",
            )
            .required("values", SyntaxShape::Any, "value field for the rose plot")
    }
    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "creates a rose plot from Nu data",
            example: "let a = ls; | plot rose $a.type $a.size",
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
        command(engine_state, stack, call)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let labels: Vec<Value> = call.req(engine_state, stack, 0)?;
    let values: Vec<Value> = call.req(engine_state, stack, 1)?;

    let label_values: Vec<String> = labels.iter().map(|x| x.as_string().unwrap()).collect();
    let values_values: Vec<i32> = values
        .iter()
        .map(|x| i32::try_from(x.as_filesize().unwrap()).unwrap())
        .collect();

    create_plot(label_values, values_values);

    Ok(PipelineData::Value(
        Value::Nothing {
            internal_span: call.head,
        },
        None,
    ))
}
