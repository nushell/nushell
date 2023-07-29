use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use crate::dataframe::values::NuExpression;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ToNu;

impl Command for ToNu {
    fn name(&self) -> &str {
        "dfr into-nu"
    }

    fn usage(&self) -> &str {
        "Converts a dataframe or an expression into into nushell value for access ane exploration."
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
            .input_output_types(vec![
                (Type::Custom("expression".into()), Type::Any),
                (Type::Custom("dataframe".into()), Type::Table(vec![])),
            ])
            //.input_output_type(Type::Any, Type::Any)
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
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr into-nu",
                result: Some(Value::List {
                    vals: vec![rec_1, rec_2],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Shows tail rows from dataframe",
                example: "[[a b]; [1 2] [5 6] [3 4]] | dfr into-df | dfr into-nu -t -n 1",
                result: Some(Value::List {
                    vals: vec![rec_3],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Convert a col expression into a nushell value",
                example: "dfr col a | dfr into-nu",
                result: Some(Value::Record {
                    cols: vec!["expr".into(), "value".into()],
                    vals: vec![Value::test_string("column"), Value::test_string("a")],
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
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            dataframe_command(engine_state, stack, call, value)
        } else {
            expression_command(call, value)
        }
    }
}

fn dataframe_command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: Value,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.get_flag(engine_state, stack, "rows")?;
    let tail: bool = call.has_flag("tail");

    let df = NuDataFrame::try_from_value(input)?;

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
fn expression_command(call: &Call, input: Value) -> Result<PipelineData, ShellError> {
    let expr = NuExpression::try_from_value(input)?;
    let value = expr.to_value(call.head);

    Ok(PipelineData::Value(value, None))
}

#[cfg(test)]
mod test {
    use super::super::super::expressions::ExprCol;
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples_dataframe_input() {
        test_dataframe(vec![Box::new(ToNu {})])
    }

    #[test]
    fn test_examples_expression_input() {
        test_dataframe(vec![Box::new(ToNu {}), Box::new(ExprCol {})])
    }
}
