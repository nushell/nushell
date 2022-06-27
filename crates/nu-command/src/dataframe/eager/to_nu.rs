use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl Command for ToNu {
    fn name(&self) -> &str {
        "into nu"
    }

    fn usage(&self) -> &str {
        "Converts a section of the dataframe into nushell Table"
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
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        let cols = vec!["index".into(), "a".into(), "b".into()];
        let rec_1 = Value::Record {
            cols: cols.clone(),
            vals: vec![Value::test_int(0), Value::test_int(1), Value::test_int(2)],
            span: Span::test_data(),
        };
        let rec_2 = Value::Record {
            cols: cols.clone(),
            vals: vec![Value::test_int(1), Value::test_int(3), Value::test_int(4)],
            span: Span::test_data(),
        };
        let rec_3 = Value::Record {
            cols,
            vals: vec![Value::test_int(2), Value::test_int(3), Value::test_int(4)],
            span: Span::test_data(),
        };

        vec![
            Example {
                description: "Shows head rows from dataframe",
                example: "[[a b]; [1 2] [3 4]] | into df | into nu",
                result: Some(Value::List {
                    vals: vec![rec_1, rec_2],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "[[a b]; [1 2] [5 6] [3 4]] | into df | into nu -t -n 1",
                result: Some(Value::List {
                    vals: vec![rec_3],
                    span: Span::test_data(),
                }),
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.get_flag(engine_state, stack, "rows")?;
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

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ToNu {})])
    }
}
