use std::collections::HashSet;
use std::iter;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    record, Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
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
        "Remove N columns at the right-hand end of the input table. To remove columns by name, use `reject`."
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
        let columns: Option<Spanned<i64>> = call.opt(engine_state, stack, 0)?;

        let columns = if let Some(columns) = columns {
            if columns.item < 0 {
                return Err(ShellError::NeedsPositiveValue(columns.span));
            } else {
                columns.item as usize
            }
        } else {
            1
        };

        Ok(drop_cols(engine_state, input, columns))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove the last column of a table",
            example: "[[lib, extension]; [nu-lib, rs] [nu-core, rb]] | drop column",
            result: Some(Value::list(
                vec![
                    Value::test_record(record!("lib" =>Value::test_string("nu-lib"))),
                    Value::test_record(record!("lib" =>Value::test_string("nu-core"))),
                ],
                Span::test_data(),
            )),
        }]
    }
}

fn drop_cols(engine_state: &EngineState, input: PipelineData, columns: usize) -> PipelineData {
    match input {
        PipelineData::ListStream(mut stream, ..) => {
            let ctrl_c = engine_state.ctrlc.clone();
            if let Some(mut first) = stream.next() {
                let drop_cols = drop_cols_set(&mut first, columns);
                if drop_cols.is_empty() {
                    iter::once(first).chain(stream).into_pipeline_data(ctrl_c)
                } else {
                    iter::once(first)
                        .chain(stream.map(move |mut v| {
                            drop_record_cols(&mut v, &drop_cols);
                            v
                        }))
                        .into_pipeline_data(ctrl_c)
                }
            } else {
                PipelineData::Empty
            }
        }
        PipelineData::Value(mut v, ..) => {
            match &mut v {
                Value::List { vals, .. } => {
                    if let Some((first, rest)) = vals.split_first_mut() {
                        let drop_cols = drop_cols_set(first, columns);
                        if !drop_cols.is_empty() {
                            for val in rest {
                                drop_record_cols(val, &drop_cols)
                            }
                        }
                    }
                }
                Value::Record { val: record, .. } => {
                    let len = record.len().saturating_sub(columns);
                    record.cols.truncate(len);
                    record.vals.truncate(len);
                }
                _ => {}
            };
            v.into_pipeline_data()
        }
        x => x,
    }
}

fn drop_cols_set(input: &mut Value, drop: usize) -> HashSet<String> {
    match input {
        Value::Record { val: record, .. } => {
            let len = record.len().saturating_sub(drop);
            record.vals.truncate(len);
            record.cols.drain(len..).collect()
        }
        _ => HashSet::new(),
    }
}

fn drop_record_cols(val: &mut Value, drop_cols: &HashSet<String>) {
    if let Value::Record { val, .. } = val {
        // TOOO: Needs `Record::retain` to be performant,
        // since this is currently O(n^2)
        // where n is the number of columns being dropped.
        // (Assuming dropped columns are at the end of the record.)
        val.retain(|col, _| !drop_cols.contains(col))
    }
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
