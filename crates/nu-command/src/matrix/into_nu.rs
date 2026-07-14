use crate::matrix::MatrixValue;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct MatrixIntoNu;

impl Command for MatrixIntoNu {
    fn name(&self) -> &str {
        "matrix into-nu"
    }

    fn signature(&self) -> Signature {
        Signature::build("matrix into-nu")
            .input_output_types(vec![(Type::Custom("matrix".into()), Type::table())])
            .switch(
                "as-records",
                "Output as a list of records with auto-generated column names",
                Some('r'),
            )
            .category(Category::Conversions)
    }

    fn description(&self) -> &str {
        "Convert a matrix to a nushell table (list of lists by default)."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "table", "list"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let as_records = call.has_flag(engine_state, stack, "as-records")?;
        let matrix = MatrixValue::from_value(&input.into_value(head)?)?;

        if matrix.array.ndim() == 0 {
            return Ok(
                Value::float(matrix.array.first().copied().unwrap_or(0.0), head)
                    .into_pipeline_data(),
            );
        }

        if as_records {
            matrix_to_records(&matrix.array, head)
        } else {
            matrix_to_lists(&matrix.array, head)
        }
    }

    fn examples(&self) -> Vec<Example<'static>> {
        vec![
            Example {
                description: "Convert an identity matrix to a nushell table",
                example: "matrix identity 2 | matrix into-nu | to nuon",
                result: Some(Value::test_string("[[1.0, 0.0], [0.0, 1.0]]")),
            },
            Example {
                description: "Convert a matrix to records",
                example: "matrix identity 2 | matrix into-nu --as-records | to nuon",
                result: Some(Value::test_string(
                    "[[\"col0\", \"col1\"]; [1.0, 0.0], [0.0, 1.0]]",
                )),
            },
        ]
    }
}

fn matrix_to_lists(array: &ndarray::ArrayD<f64>, span: Span) -> Result<PipelineData, ShellError> {
    let rows = match array.ndim() {
        0 => {
            vec![Value::float(array.first().copied().unwrap_or(0.0), span)]
        }
        1 => {
            let vals: Vec<Value> = array.iter().map(|v| Value::float(*v, span)).collect();
            vec![Value::list(vals, span)]
        }
        2 => array
            .axis_iter(ndarray::Axis(0))
            .map(|row| {
                let vals: Vec<Value> = row.iter().map(|v| Value::float(*v, span)).collect();
                Value::list(vals, span)
            })
            .collect(),
        _ => array
            .axis_iter(ndarray::Axis(0))
            .map(|sub| {
                let sub_lists: Vec<Value> = sub
                    .axis_iter(ndarray::Axis(0))
                    .map(|inner| {
                        let vals: Vec<Value> =
                            inner.iter().map(|v| Value::float(*v, span)).collect();
                        Value::list(vals, span)
                    })
                    .collect();
                Value::list(sub_lists, span)
            })
            .collect(),
    };

    Ok(Value::list(rows, span).into_pipeline_data())
}

fn matrix_to_records(array: &ndarray::ArrayD<f64>, span: Span) -> Result<PipelineData, ShellError> {
    let ncols = if array.ndim() >= 2 {
        array.shape()[array.ndim() - 1]
    } else {
        array.len()
    };
    let col_names: Vec<String> = (0..ncols).map(|i| format!("col{}", i)).collect();

    let rows: Vec<Value> = array
        .axis_iter(ndarray::Axis(0))
        .map(|row| {
            let mut record = nu_protocol::Record::new();
            for (j, name) in col_names.iter().enumerate() {
                let val = if j < row.len() {
                    if let Some(&v) = row.get(j) {
                        Value::float(v, span)
                    } else {
                        Value::float(0.0, span)
                    }
                } else {
                    Value::float(0.0, span)
                };
                record.push(name, val);
            }
            Value::record(record, span)
        })
        .collect();

    Ok(Value::list(rows, span).into_pipeline_data())
}
