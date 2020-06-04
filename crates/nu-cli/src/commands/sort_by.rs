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

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sort_by(args, registry).await
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

async fn sort_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let tag = args.call_info.name_tag.clone();

    let (SortByArgs { rest }, mut input) = args.process(&registry).await?;
    let mut vec = input.drain_vec().await;

    if vec.is_empty() {
        return Err(ShellError::labeled_error(
            "Error performing sort-by command",
            "sort-by error",
            tag,
        ));
    }

    for sort_arg in rest.iter() {
        let match_test = get_data_by_key(&vec[0], sort_arg.borrow_spanned());
        if match_test == None {
            return Err(ShellError::labeled_error(
                "Can not find column to sort by",
                "invalid column",
                sort_arg.borrow_spanned().span,
            ));
        }
    }

    match &vec[0] {
        Value {
            value: UntaggedValue::Primitive(_),
            ..
        } => {
            vec.sort();
        }
        _ => {
            let calc_key = |item: &Value| {
                rest.iter()
                    .map(|f| get_data_by_key(item, f.borrow_spanned()))
                    .collect::<Vec<Option<Value>>>()
            };
            vec.sort_by_cached_key(calc_key);
        }
    };

    let mut values_vec_deque: VecDeque<Value> = VecDeque::new();

    for item in vec {
        values_vec_deque.push_back(item);
    }

    Ok(futures::stream::iter(values_vec_deque).to_output_stream())
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
