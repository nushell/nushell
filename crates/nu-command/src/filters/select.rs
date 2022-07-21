use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineIterator, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Select;

impl Command for Select {
    fn name(&self) -> &str {
        "select"
    }

    // FIXME: also add support for --skip
    fn signature(&self) -> Signature {
        Signature::build("select")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the columns to select from the table",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["pick", "choose", "get"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let span = call.head;

        select(engine_state, span, columns, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Select just the name column",
                example: "ls | select name",
                result: None,
            },
            Example {
                description: "Select the name and size columns",
                example: "ls | select name size",
                result: None,
            },
        ]
    }
}

fn select(
    engine_state: &EngineState,
    span: Span,
    columns: Vec<CellPath>,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let mut rows = vec![];

    let mut new_columns = vec![];

    for column in columns {
        let CellPath { ref members } = column;
        match members.get(0) {
            Some(PathMember::Int { val, span }) => {
                if members.len() > 1 {
                    return Err(ShellError::GenericError(
                        "Select only allows row numbers for rows".into(),
                        "extra after row number".into(),
                        Some(*span),
                        None,
                        Vec::new(),
                    ));
                }

                rows.push(*val);
            }
            _ => new_columns.push(column),
        };
    }
    let columns = new_columns;

    let input = if !rows.is_empty() {
        rows.sort_unstable();
        // let skip = call.has_flag("skip");
        let metadata = input.metadata();
        let pipeline_iter: PipelineIterator = input.into_iter();

        NthIterator {
            input: pipeline_iter,
            rows,
            skip: false,
            current: 0,
        }
        .into_pipeline_data(engine_state.ctrlc.clone())
        .set_metadata(metadata)
    } else {
        input
    };

    match input {
        PipelineData::Value(
            Value::List {
                vals: input_vals,
                span,
            },
            metadata,
            ..,
        ) => {
            let mut output = vec![];

            for input_val in input_vals {
                if !columns.is_empty() {
                    let mut cols = vec![];
                    let mut vals = vec![];
                    for path in &columns {
                        //FIXME: improve implementation to not clone
                        let fetcher = input_val.clone().follow_cell_path(&path.members, false)?;

                        cols.push(path.into_string().replace('.', "_"));
                        vals.push(fetcher);
                    }

                    output.push(Value::Record { cols, vals, span })
                } else {
                    output.push(input_val)
                }
            }

            Ok(output
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone())
                .set_metadata(metadata))
        }
        PipelineData::ListStream(stream, metadata, ..) => Ok(stream
            .map(move |x| {
                if !columns.is_empty() {
                    let mut cols = vec![];
                    let mut vals = vec![];
                    for path in &columns {
                        //FIXME: improve implementation to not clone
                        match x.clone().follow_cell_path(&path.members, false) {
                            Ok(value) => {
                                cols.push(path.into_string().replace('.', "_"));
                                vals.push(value);
                            }
                            Err(_) => {
                                cols.push(path.into_string().replace('.', "_"));
                                vals.push(Value::Nothing { span });
                            }
                        }
                    }
                    Value::Record { cols, vals, span }
                } else {
                    x
                }
            })
            .into_pipeline_data(engine_state.ctrlc.clone())
            .set_metadata(metadata)),
        PipelineData::Value(v, metadata, ..) => {
            if !columns.is_empty() {
                let mut cols = vec![];
                let mut vals = vec![];

                for cell_path in columns {
                    // FIXME: remove clone
                    let result = v.clone().follow_cell_path(&cell_path.members, false)?;

                    cols.push(cell_path.into_string().replace('.', "_"));
                    vals.push(result);
                }

                Ok(Value::Record { cols, vals, span }
                    .into_pipeline_data()
                    .set_metadata(metadata))
            } else {
                Ok(v.into_pipeline_data().set_metadata(metadata))
            }
        }
        _ => Ok(PipelineData::new(span)),
    }
}

struct NthIterator {
    input: PipelineIterator,
    rows: Vec<usize>,
    skip: bool,
    current: usize,
}

impl Iterator for NthIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.skip {
                if let Some(row) = self.rows.get(0) {
                    if self.current == *row {
                        self.rows.remove(0);
                        self.current += 1;
                        return self.input.next();
                    } else {
                        self.current += 1;
                        let _ = self.input.next();
                        continue;
                    }
                } else {
                    return None;
                }
            } else if let Some(row) = self.rows.get(0) {
                if self.current == *row {
                    self.rows.remove(0);
                    self.current += 1;
                    let _ = self.input.next();
                    continue;
                } else {
                    self.current += 1;
                    return self.input.next();
                }
            } else {
                return self.input.next();
            }
        }
    }
}
