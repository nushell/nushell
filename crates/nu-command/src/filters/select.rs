use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{
    Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Select;

impl Command for Select {
    fn name(&self) -> &str {
        "select"
    }

    fn signature(&self) -> Signature {
        Signature::build("select").rest(
            "rest",
            SyntaxShape::CellPath,
            "the columns to select from the table",
        )
    }

    fn usage(&self) -> &str {
        "Down-select table to only these columns."
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let columns: Vec<CellPath> = call.rest(context, 0)?;
        let span = call.head;

        select(span, columns, input)
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
    span: Span,
    columns: Vec<CellPath>,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    if columns.is_empty() {
        return Err(ShellError::CantFindColumn(span, span)); //FIXME?
    }

    match input {
        PipelineData::Value(Value::List {
            vals: input_vals,
            span,
        }) => {
            let mut output = vec![];

            for input_val in input_vals {
                let mut cols = vec![];
                let mut vals = vec![];
                for path in &columns {
                    //FIXME: improve implementation to not clone
                    let fetcher = input_val.clone().follow_cell_path(&path.members)?;

                    cols.push(path.into_string());
                    vals.push(fetcher);
                }

                output.push(Value::Record { cols, vals, span })
            }

            Ok(output.into_iter().into_pipeline_data())
        }
        PipelineData::Stream(stream) => Ok(stream
            .map(move |x| {
                let mut cols = vec![];
                let mut vals = vec![];
                for path in &columns {
                    //FIXME: improve implementation to not clone
                    match x.clone().follow_cell_path(&path.members) {
                        Ok(value) => {
                            cols.push(path.into_string());
                            vals.push(value);
                        }
                        Err(error) => {
                            cols.push(path.into_string());
                            vals.push(Value::Error { error });
                        }
                    }
                }

                Value::Record { cols, vals, span }
            })
            .into_pipeline_data()),
        PipelineData::Value(v) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for cell_path in columns {
                // FIXME: remove clone
                let result = v.clone().follow_cell_path(&cell_path.members)?;

                cols.push(cell_path.into_string());
                vals.push(result);
            }

            Ok(Value::Record { cols, vals, span }.into_pipeline_data())
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use nu_protocol::ColumnPath;
//     use nu_source::Span;
//     use nu_source::SpannedItem;
//     use nu_source::Tag;
//     use nu_stream::InputStream;
//     use nu_test_support::value::nothing;
//     use nu_test_support::value::row;
//     use nu_test_support::value::string;

//     use super::select;
//     use super::Command;
//     use super::ShellError;

//     #[test]
//     fn examples_work_as_expected() -> Result<(), ShellError> {
//         use crate::examples::test as test_examples;

//         test_examples(Command {})
//     }

//     #[test]
//     fn select_using_sparse_table() {
//         // Create a sparse table with 3 rows:
//         //   col_foo | col_bar
//         //   -----------------
//         //   foo     |
//         //           | bar
//         //   foo     |
//         let input = vec![
//             row(indexmap! {"col_foo".into() => string("foo")}),
//             row(indexmap! {"col_bar".into() => string("bar")}),
//             row(indexmap! {"col_foo".into() => string("foo")}),
//         ];

//         let expected = vec![
//             row(
//                 indexmap! {"col_none".into() => nothing(), "col_foo".into() => string("foo"), "col_bar".into() => nothing()},
//             ),
//             row(
//                 indexmap! {"col_none".into() => nothing(), "col_foo".into() => nothing(), "col_bar".into() => string("bar")},
//             ),
//             row(
//                 indexmap! {"col_none".into() => nothing(), "col_foo".into() => string("foo"), "col_bar".into() => nothing()},
//             ),
//         ];

//         let actual = select(
//             Tag::unknown(),
//             vec![
//                 ColumnPath::build(&"col_none".to_string().spanned(Span::unknown())),
//                 ColumnPath::build(&"col_foo".to_string().spanned(Span::unknown())),
//                 ColumnPath::build(&"col_bar".to_string().spanned(Span::unknown())),
//             ],
//             input.into(),
//         );

//         assert_eq!(Ok(expected), actual.map(InputStream::into_vec));
//     }
// }
