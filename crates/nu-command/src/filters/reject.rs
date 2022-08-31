use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, FromValue, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Reject;

impl Command for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject")
            .rest(
                "rest",
                SyntaxShape::String,
                "the names of columns to remove from the table",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove the given columns from the table. If you want to remove rows, try 'drop'."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let columns: Vec<String> = call.rest(engine_state, stack, 0)?;
        let span = call.head;
        reject(engine_state, span, input, columns)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Lists the files in a directory without showing the modified column",
                example: "ls | reject modified",
                result: None,
            },
            Example {
                description: "Reject the specified field in a record",
                example: "echo {a: 1, b: 2} | reject a",
                result: Some(Value::Record {
                    cols: vec!["b".into()],
                    vals: vec![Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn reject(
    engine_state: &EngineState,
    span: Span,
    input: PipelineData,
    columns: Vec<String>,
) -> Result<PipelineData, ShellError> {
    if columns.is_empty() {
        return Err(ShellError::CantFindColumn(span, span));
    }
    let metadata = input.metadata();

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
                    let fetcher = input_val.clone().follow_cell_path(&path.members, false);

                    if let Ok(value) = fetcher {
                        cols.push(path.into_string());
                        vals.push(value);
                    }
                }
                output.push(Value::Record { cols, vals, span })
            }

            Ok(output
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        }
        PipelineData::Value(
            Value::Record {
                mut cols,
                mut vals,
                span,
            },
            metadata,
        ) => {
            reject_record_columns(&mut cols, &mut vals, &columns);

            let record = Value::Record { cols, vals, span };

            Ok(PipelineData::Value(record, metadata))
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
    .map(|x| x.set_metadata(metadata))
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

fn get_keep_columns(mut input: Vec<String>, rejects: Vec<String>) -> Vec<String> {
    for reject in rejects {
        if let Some(index) = input.iter().position(|value| *value == reject) {
            input.remove(index);
        }
    }
    input
}

fn reject_record_columns(cols: &mut Vec<String>, vals: &mut Vec<Value>, rejects: &[String]) {
    for reject in rejects {
        if let Some(index) = cols.iter().position(|value| value == reject) {
            cols.remove(index);
            vals.remove(index);
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Reject;
        use crate::test_examples;
        test_examples(Reject {})
    }
}
