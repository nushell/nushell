use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::{Block, Call};
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    PipelineIterator, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Clone)]
pub struct UpdateCells;

impl Command for UpdateCells {
    fn name(&self) -> &str {
        "update cells"
    }

    fn signature(&self) -> Signature {
        Signature::build("update cells")
            .input_output_types(vec![(Type::Table(vec![]), Type::Table(vec![]))])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run an update for each cell",
            )
            .named(
                "columns",
                SyntaxShape::Table,
                "list of columns to update",
                Some('c'),
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Update the table cells."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update the zero value cells to empty strings.",
                example: r#"[
    ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
    [          37,            0,            0,            0,           37,            0,            0]
] | update cells { |value|
      if $value == 0 {
        ""
      } else {
        $value
      }
}"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec![
                            "2021-04-16".into(),
                            "2021-06-10".into(),
                            "2021-09-18".into(),
                            "2021-10-15".into(),
                            "2021-11-16".into(),
                            "2021-11-17".into(),
                            "2021-11-18".into(),
                        ],
                        vals: vec![
                            Value::test_int(37),
                            Value::test_string(""),
                            Value::test_string(""),
                            Value::test_string(""),
                            Value::test_int(37),
                            Value::test_string(""),
                            Value::test_string(""),
                        ],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Update the zero value cells to empty strings in 2 last columns.",
                example: r#"[
    ["2021-04-16", "2021-06-10", "2021-09-18", "2021-10-15", "2021-11-16", "2021-11-17", "2021-11-18"];
    [          37,            0,            0,            0,           37,            0,            0]
] | update cells -c ["2021-11-18", "2021-11-17"] { |value|
        if $value == 0 {
          ""
        } else {
          $value
        }
}"#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec![
                            "2021-04-16".into(),
                            "2021-06-10".into(),
                            "2021-09-18".into(),
                            "2021-10-15".into(),
                            "2021-11-16".into(),
                            "2021-11-17".into(),
                            "2021-11-18".into(),
                        ],
                        vals: vec![
                            Value::test_int(37),
                            Value::test_int(0),
                            Value::test_int(0),
                            Value::test_int(0),
                            Value::test_int(37),
                            Value::test_string(""),
                            Value::test_string(""),
                        ],
                        span: Span::test_data(),
                    }],
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
        // the block to run on each cell
        let engine_state = engine_state.clone();
        let block: Closure = call.req(&engine_state, stack, 0)?;
        let mut stack = stack.captures_to_stack(&block.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        let metadata = input.metadata();
        let ctrlc = engine_state.ctrlc.clone();
        let block: Block = engine_state.get_block(block.block_id).clone();

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let span = call.head;

        stack.with_env(&orig_env_vars, &orig_env_hidden);

        // the columns to update
        let columns: Option<Value> = call.get_flag(&engine_state, &mut stack, "columns")?;
        let columns: Option<HashSet<String>> = match columns {
            Some(val) => {
                let cols = val
                    .as_list()?
                    .iter()
                    .map(|val| val.as_string())
                    .collect::<Result<Vec<String>, ShellError>>()?;
                Some(HashSet::from_iter(cols.into_iter()))
            }
            None => None,
        };

        Ok(UpdateCellIterator {
            input: input.into_iter(),
            engine_state,
            stack,
            block,
            columns,
            redirect_stdout,
            redirect_stderr,
            span,
        }
        .into_pipeline_data(ctrlc)
        .set_metadata(metadata))
    }
}

struct UpdateCellIterator {
    input: PipelineIterator,
    columns: Option<HashSet<String>>,
    engine_state: EngineState,
    stack: Stack,
    block: Block,
    redirect_stdout: bool,
    redirect_stderr: bool,
    span: Span,
}

impl Iterator for UpdateCellIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.next() {
            Some(val) => {
                if let Some(ref cols) = self.columns {
                    if !val.columns().iter().any(|c| cols.contains(c)) {
                        return Some(val);
                    }
                }

                match val {
                    Value::Record { vals, cols, span } => Some(Value::Record {
                        vals: cols
                            .iter()
                            .zip(vals.into_iter())
                            .map(|(col, val)| match &self.columns {
                                Some(cols) if !cols.contains(col) => val,
                                _ => process_cell(
                                    val,
                                    &self.engine_state,
                                    &mut self.stack,
                                    &self.block,
                                    self.redirect_stdout,
                                    self.redirect_stderr,
                                    span,
                                ),
                            })
                            .collect(),
                        cols,
                        span,
                    }),
                    val => Some(process_cell(
                        val,
                        &self.engine_state,
                        &mut self.stack,
                        &self.block,
                        self.redirect_stdout,
                        self.redirect_stderr,
                        self.span,
                    )),
                }
            }
            None => None,
        }
    }
}

fn process_cell(
    val: Value,
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    redirect_stdout: bool,
    redirect_stderr: bool,
    span: Span,
) -> Value {
    if let Some(var) = block.signature.get_positional(0) {
        if let Some(var_id) = &var.var_id {
            stack.add_var(*var_id, val.clone());
        }
    }
    match eval_block(
        engine_state,
        stack,
        block,
        val.into_pipeline_data(),
        redirect_stdout,
        redirect_stderr,
    ) {
        Ok(pd) => pd.into_value(span),
        Err(e) => Value::Error { error: e },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(UpdateCells {})
    }
}
