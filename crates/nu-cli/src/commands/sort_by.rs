use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use nu_value_ext::get_data_by_key;

pub struct SortBy;

#[derive(Deserialize)]
pub struct SortByArgs {
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for SortBy {
    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("sort-by").rest(SyntaxShape::String, "the column(s) to sort by")
    }

    fn usage(&self) -> &str {
        "Sort by the given columns, in increasing order."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sort_by(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sort list by increasing value",
                example: "echo [4 2 3 1] | sort-by",
                result: Some(vec![
                    UntaggedValue::int(1).into(),
                    UntaggedValue::int(2).into(),
                    UntaggedValue::int(3).into(),
                    UntaggedValue::int(4).into(),
                ]),
            },
            Example {
                description: "Sort output by increasing file size",
                example: "ls | sort-by size",
                result: None,
            },
            Example {
                description: "Sort output by type, and then by file size for each type",
                example: "ls | sort-by type size",
                result: None,
            },
        ]
    }
}

fn sort_by(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let (SortByArgs { rest }, mut input) = args.process(&registry).await?;
        let mut vec = input.drain_vec().await;

        if vec.is_empty() {
            return;
        }

        for sort_arg in rest.iter() {
            let match_test = get_data_by_key(&vec[0], sort_arg.borrow_spanned());
            if match_test == None {
                yield Err(ShellError::labeled_error(
                    "Can not find column to sort by",
                    "invalid column",
                    sort_arg.borrow_spanned().span,
                ));
                return;
            }
        }

        match &vec[0] {
            Value {
                value: UntaggedValue::Primitive(_),
                ..
            } => {
                vec.sort();
            },
            _ => {
                let calc_key = |item: &Value| {
                    rest.iter()
                        .map(|f| get_data_by_key(item, f.borrow_spanned()))
                        .collect::<Vec<Option<Value>>>()
                };
                vec.sort_by_cached_key(calc_key);
            },
        };

        for item in vec {
            yield item.into();
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::SortBy;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SortBy {})
    }
}
