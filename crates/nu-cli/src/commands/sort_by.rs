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

    fn examples(&self) -> &[Example] {
        &[
            Example {
                description: "Sort output by increasing file size",
                example: "ls | sort-by size",
            },
            Example {
                description: "Sort output by type, and then by file size for each type",
                example: "ls | sort-by type size",
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
