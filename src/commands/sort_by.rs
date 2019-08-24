use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct SortBy;

impl WholeStreamCommand for SortBy {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        sort_by(args, registry)
    }

    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("sort-by").switch("reverse")
    }
}

fn sort_by(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let (input, args) = args.parts();

    let fields: Result<Vec<_>, _> = args
        .positional
        .iter()
        .flatten()
        .map(|a| a.as_string())
        .collect();

    let fields = fields?;

    let output = input.values.collect::<Vec<_>>();

    let reverse = args.has("reverse");
    let output = output.map(move |mut vec| {
        let calc_key = |item: &Tagged<Value>| {
            fields
                .iter()
                .map(|f| item.get_data_by_key(f).map(|i| i.clone()))
                .collect::<Vec<Option<Tagged<Value>>>>()
        };
        if reverse {
            vec.sort_by_cached_key(|item| {
                std::cmp::Reverse(calc_key(item))
            });
        } else {
            vec.sort_by_cached_key(calc_key);
        }

        vec.into_iter().collect::<VecDeque<_>>()
    });

    Ok(output.flatten_stream().from_input_stream())
}
