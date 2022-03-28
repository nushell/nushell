use nu_engine::column::get_columns;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Transpose;

pub struct TransposeArgs {
    rest: Vec<Spanned<String>>,
    header_row: bool,
    ignore_titles: bool,
}

impl Command for Transpose {
    fn name(&self) -> &str {
        "transpose"
    }

    fn signature(&self) -> Signature {
        Signature::build("transpose")
            .switch(
                "header-row",
                "treat the first row as column names",
                Some('r'),
            )
            .switch(
                "ignore-titles",
                "don't transpose the column names into values",
                Some('i'),
            )
            .rest(
                "rest",
                SyntaxShape::String,
                "the names to give columns once transposed",
            )
    }

    fn usage(&self) -> &str {
        "Transposes the table contents so rows become columns and columns become rows."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        transpose(engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        let span = Span::test_data();
        vec![
            Example {
                description: "Transposes the table contents with default column names",
                example: "echo [[c1 c2]; [1 2]] | transpose",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("c1"), Value::test_int(1)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["column0".to_string(), "column1".to_string()],
                            vals: vec![Value::test_string("c2"), Value::test_int(2)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description: "Transposes the table contents with specified column names",
                example: "echo [[c1 c2]; [1 2]] | transpose key val",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["key".to_string(), "val".to_string()],
                            vals: vec![Value::test_string("c1"), Value::test_int(1)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["key".to_string(), "val".to_string()],
                            vals: vec![Value::test_string("c2"), Value::test_int(2)],
                            span,
                        },
                    ],
                    span,
                }),
            },
            Example {
                description:
                    "Transposes the table without column names and specify a new column name",
                example: "echo [[c1 c2]; [1 2]] | transpose -i val",
                result: Some(Value::List {
                    vals: vec![
                        Value::Record {
                            cols: vec!["val".to_string()],
                            vals: vec![Value::test_int(1)],
                            span,
                        },
                        Value::Record {
                            cols: vec!["val".to_string()],
                            vals: vec![Value::test_int(2)],
                            span,
                        },
                    ],
                    span,
                }),
            },
        ]
    }
}

pub fn transpose(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name = call.head;
    let transpose_args = TransposeArgs {
        header_row: call.has_flag("header-row"),
        ignore_titles: call.has_flag("ignore-titles"),
        rest: call.rest(engine_state, stack, 0)?,
    };

    let ctrlc = engine_state.ctrlc.clone();
    let metadata = input.metadata();
    let input: Vec<_> = input.into_iter().collect();
    let args = transpose_args;

    let descs = get_columns(&input);

    let mut headers: Vec<String> = vec![];

    if !args.rest.is_empty() && args.header_row {
        return Err(ShellError::SpannedLabeledError(
            "Can not provide header names and use header row".into(),
            "using header row".into(),
            name,
        ));
    }

    if args.header_row {
        for i in input.clone() {
            if let Some(desc) = descs.get(0) {
                match &i.get_data_by_key(desc) {
                    Some(x) => {
                        if let Ok(s) = x.as_string() {
                            headers.push(s.to_string());
                        } else {
                            return Err(ShellError::SpannedLabeledError(
                                "Header row needs string headers".into(),
                                "used non-string headers".into(),
                                name,
                            ));
                        }
                    }
                    _ => {
                        return Err(ShellError::SpannedLabeledError(
                            "Header row is incomplete and can't be used".into(),
                            "using incomplete header row".into(),
                            name,
                        ));
                    }
                }
            } else {
                return Err(ShellError::SpannedLabeledError(
                    "Header row is incomplete and can't be used".into(),
                    "using incomplete header row".into(),
                    name,
                ));
            }
        }
    } else {
        for i in 0..=input.len() {
            if let Some(name) = args.rest.get(i) {
                headers.push(name.item.clone())
            } else {
                headers.push(format!("column{}", i));
            }
        }
    }

    let descs: Vec<_> = if args.header_row {
        descs.into_iter().skip(1).collect()
    } else {
        descs
    };

    Ok((descs.into_iter().map(move |desc| {
        let mut column_num: usize = 0;
        let mut cols = vec![];
        let mut vals = vec![];

        if !args.ignore_titles && !args.header_row {
            cols.push(headers[column_num].clone());
            vals.push(Value::string(desc.clone(), name));
            column_num += 1
        }

        for i in input.clone() {
            match &i.get_data_by_key(&desc) {
                Some(x) => {
                    cols.push(headers[column_num].clone());
                    vals.push(x.clone());
                }
                _ => {
                    cols.push(headers[column_num].clone());
                    vals.push(Value::nothing(name));
                }
            }
            column_num += 1;
        }

        Value::Record {
            cols,
            vals,
            span: name,
        }
    }))
    .into_pipeline_data(ctrlc).set_metadata(metadata))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Transpose {})
    }
}
