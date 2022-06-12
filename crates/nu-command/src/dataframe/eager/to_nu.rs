use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl Command for ToNu {
    fn name(&self) -> &str {
        "to nu"
    }

    fn usage(&self) -> &str {
        "Converts a section of the dataframe to Nushell Table"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "rows",
                SyntaxShape::Number,
                "number of rows to be shown",
                Some('n'),
            )
            .switch("tail", "shows tail rows", Some('t'))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Shows head rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | to-df | to nu",
                result: None,
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "[[a b]; [1 2] [3 4] [5 6]] | to-df | to nu -t -n 1",
                result: None,
            },
        ]
    }

    fn input_type(&self) -> Type {
        Type::Custom("dataframe".into())
    }

    fn output_type(&self) -> Type {
        Type::Any
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
    let rows: Option<usize> = call.get_flag(engine_state, stack, "n-rows")?;
    let tail: bool = call.has_flag("tail");

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let values = if tail {
        df.tail(rows, call.head)?
    } else {
        // if rows is specified, return those rows, otherwise return everything
        if rows.is_some() {
            df.head(rows, call.head)?
        } else {
            df.head(Some(df.height()), call.head)?
        }
    };

    let value = Value::List {
        vals: values,
        span: call.head,
    };

    Ok(PipelineData::Value(value, None))
}
