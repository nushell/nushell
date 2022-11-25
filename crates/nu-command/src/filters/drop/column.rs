use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct DropColumn;

impl Command for DropColumn {
    fn name(&self) -> &str {
        "drop column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .optional(
                "columns",
                SyntaxShape::Int,
                "starting from the end, the number of columns to remove",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove N columns at the right-hand end of the input table. To remove columns by name, use 'reject'."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // the number of columns to drop
        let columns: Option<i64> = call.opt(engine_state, stack, 0)?;
        let span = call.head;

        let columns_to_drop = if let Some(quantity) = columns {
            quantity
        } else {
            1
        };

        // Make columns to drop is positive
        if columns_to_drop < 0 {
            if let Some(expr) = call.positional_nth(0) {
                return Err(ShellError::NeedsPositiveValue(expr.span));
            }
        }

        dropcol(engine_state, span, input, columns_to_drop)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the last column of a table",
            example: "echo [[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column",
            result: Some(Value::List {
                vals: vec![
                    Value::Record {
                        cols: vec!["lib".into()],
                        vals: vec![Value::test_string("nu-lib")],
                        span: Span::test_data(),
                    },
                    Value::Record {
                        cols: vec!["lib".into()],
                        vals: vec![Value::test_string("nu-core")],
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }
}

fn dropcol(
    engine_state: &EngineState,
    span: Span,
    input: PipelineData,
    columns: i64, // the number of columns to drop
) -> Result<PipelineData, ShellError> {
    let mut keep_columns = vec![];

    match input {
        PipelineData::Value(
            Value::List {
                vals: input_vals,
                span,
            },
            ..,
        ) => {
            let mut output = vec![];
            let input_cols = get_input_cols(input_vals.clone());
            let kc = get_keep_columns(input_cols, columns);
            keep_columns = get_cellpath_columns(kc, span);

            for input_val in input_vals {
                let mut cols = vec![];
                let mut vals = vec![];

                for path in &keep_columns {
                    let fetcher = input_val.clone().follow_cell_path(&path.members, false)?;
                    cols.push(path.into_string());
                    vals.push(fetcher);
                }
                output.push(Value::Record { cols, vals, span })
            }

            Ok(output
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::ListStream(stream, ..) => {
            let mut output = vec![];

            let v: Vec<_> = stream.into_iter().collect();
            let input_cols = get_input_cols(v.clone());
            let kc = get_keep_columns(input_cols, columns);
            keep_columns = get_cellpath_columns(kc, span);

            for input_val in v {
                let mut cols = vec![];
                let mut vals = vec![];

                for path in &keep_columns {
                    let fetcher = input_val.clone().follow_cell_path(&path.members, false)?;
                    cols.push(path.into_string());
                    vals.push(fetcher);
                }
                output.push(Value::Record { cols, vals, span })
            }

            Ok(output
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(v, ..) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for cell_path in &keep_columns {
                let result = v.clone().follow_cell_path(&cell_path.members, false)?;

                cols.push(cell_path.into_string());
                vals.push(result);
            }

            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
        x => Ok(x),
    }
}

fn get_input_cols(input: Vec<Value>) -> Vec<String> {
    let rec = input.first();
    match rec {
        Some(Value::Record { cols, vals: _, .. }) => cols.to_vec(),
        _ => vec!["".to_string()],
    }
}

fn get_cellpath_columns(keep_cols: Vec<String>, span: Span) -> Vec<CellPath> {
    let mut output = vec![];
    for keep_col in keep_cols {
        let val = Value::String {
            val: keep_col,
            span,
        };
        let cell_path = match CellPath::from_value(&val) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        output.push(cell_path);
    }
    output
}

fn get_keep_columns(input: Vec<String>, mut num_of_columns_to_drop: i64) -> Vec<String> {
    let vlen: i64 = input.len() as i64;

    if num_of_columns_to_drop > vlen {
        num_of_columns_to_drop = vlen;
    }

    let num_of_columns_to_keep = (vlen - num_of_columns_to_drop) as usize;
    input[0..num_of_columns_to_keep].to_vec()
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::DropColumn;
        use crate::test_examples;
        test_examples(DropColumn {})
    }
}
