use crate::prelude::*;
use nu_engine::run_block;
use nu_engine::WholeStreamCommand;

use nu_errors::ShellError;
use nu_protocol::{
    hir::{CapturedBlock, ExternalRedirection},
    Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use std::collections::HashSet;
use std::iter::FromIterator;

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "update cells"
    }

    fn signature(&self) -> Signature {
        Signature::build("update cells")
            .required(
                "block",
                SyntaxShape::Block,
                "the block to run an update for each cell",
            )
            .named(
                "columns",
                SyntaxShape::Table,
                "list of columns to update",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Update the table cells."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        update_cells(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Update the zero value cells to empty strings.",
                example: r#"[
  [2021-04-16, 2021-06-10, 2021-09-18, 2021-10-15, 2021-11-16, 2021-11-17, 2021-11-18];
  [        37,          0,          0,          0,         37,          0,          0]
] | update cells {|value|
      if ($value | into int) == 0 {
        ""
      } {
        $value
      }
}"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                        "2021-04-16".to_string() => UntaggedValue::int(37).into(),
                        "2021-06-10".to_string() => Value::from(""),
                        "2021-09-18".to_string() => Value::from(""),
                        "2021-10-15".to_string() => Value::from(""),
                        "2021-11-16".to_string() => UntaggedValue::int(37).into(),
                        "2021-11-17".to_string() => Value::from(""),
                        "2021-11-18".to_string() => Value::from(""),
                })
                .into()]),
            },
            Example {
                description: "Update the zero value cells to empty strings in 2 last columns.",
                example: r#"[
    [2021-04-16, 2021-06-10, 2021-09-18, 2021-10-15, 2021-11-16, 2021-11-17, 2021-11-18];
    [        37,          0,          0,          0,         37,          0,          0]
] | update cells -c ["2021-11-18", "2021-11-17"] {|value|
        if ($value | into int) == 0 {
        ""
        } {
        $value
        }
}"#,
                result: Some(vec![UntaggedValue::row(indexmap! {
                        "2021-04-16".to_string() => UntaggedValue::int(37).into(),
                        "2021-06-10".to_string() => UntaggedValue::int(0).into(),
                        "2021-09-18".to_string() => UntaggedValue::int(0).into(),
                        "2021-10-15".to_string() => UntaggedValue::int(0).into(),
                        "2021-11-16".to_string() => UntaggedValue::int(37).into(),
                        "2021-11-17".to_string() => Value::from(""),
                        "2021-11-18".to_string() => Value::from(""),
                })
                .into()]),
            },
        ]
    }
}

fn update_cells(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let context = Arc::new(args.context.clone());
    let external_redirection = args.call_info.args.external_redirection;

    let block: CapturedBlock = args.req(0)?;
    let block = Arc::new(block);

    let columns = args
        .get_flag("columns")?
        .map(|x: Value| HashSet::from_iter(x.table_entries().map(|val| val.convert_to_string())));
    let columns = Arc::new(columns);

    Ok(args
        .input
        .flat_map(move |input| {
            let block = block.clone();
            let context = context.clone();

            if input.is_row() {
                OutputStream::one(process_cells(
                    block,
                    columns.clone(),
                    context,
                    input,
                    external_redirection,
                ))
            } else {
                match process_input(block, context, input, external_redirection) {
                    Ok(s) => s,
                    Err(e) => OutputStream::one(Value::error(e)),
                }
            }
        })
        .into_output_stream())
}

pub fn process_input(
    captured_block: Arc<CapturedBlock>,
    context: Arc<EvaluationContext>,
    input: Value,
    external_redirection: ExternalRedirection,
) -> Result<OutputStream, ShellError> {
    let input_clone = input.clone();
    // When we process a row, we need to know whether the block wants to have the contents of the row as
    // a parameter to the block (so it gets assigned to a variable that can be used inside the block) or
    // if it wants the contents as as an input stream

    let input_stream = if !captured_block.block.params.positional.is_empty() {
        InputStream::empty()
    } else {
        vec![Ok(input_clone)].into_iter().into_input_stream()
    };

    context.scope.enter_scope();
    context.scope.add_vars(&captured_block.captured.entries);

    if let Some((arg, _)) = captured_block.block.params.positional.first() {
        context.scope.add_var(arg.name(), input);
    } else {
        context.scope.add_var("$it", input);
    }

    let result = run_block(
        &captured_block.block,
        &context,
        input_stream,
        external_redirection,
    );

    context.scope.exit_scope();

    result
}

pub fn process_cells(
    captured_block: Arc<CapturedBlock>,
    columns: Arc<Option<HashSet<String>>>,
    context: Arc<EvaluationContext>,
    input: Value,
    external_redirection: ExternalRedirection,
) -> Value {
    TaggedDictBuilder::build(input.tag(), |row| {
        input.row_entries().for_each(|(column, cell_value)| {
            match &*columns {
                Some(col) if !col.contains(column) => {
                    row.insert_value(column, cell_value.clone());
                    return;
                }
                _ => {}
            };
            let cell_processed = process_input(
                captured_block.clone(),
                context.clone(),
                cell_value.clone(),
                external_redirection,
            )
            .map(|it| it.into_vec())
            .map_err(Value::error);

            match cell_processed {
                Ok(value) => {
                    match value.get(0) {
                        Some(one) => {
                            row.insert_value(column, one.clone());
                        }
                        None => {
                            row.insert_untagged(column, UntaggedValue::nothing());
                        }
                    };
                }
                Err(reason) => {
                    row.insert_value(column, reason);
                }
            }
        });
    })
}
