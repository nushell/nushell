use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct SortBy;

#[derive(Deserialize)]
pub struct SortByArgs {
    rest: Vec<Tagged<String>>,
    reverse: bool,
}

impl WholeStreamCommand for SortBy {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, sort_by)?.run()
    }

    fn name(&self) -> &str {
        "sort-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("sort-by")
            .rest(SyntaxType::String)
            .switch("reverse")
    }
}

fn sort_by(
    SortByArgs { reverse, rest }: SortByArgs,
    mut context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream_block! {
        let mut vec = context.input.drain_vec().await;

        let calc_key = |item: &Tagged<Value>| {
            rest.iter()
                .map(|f| item.get_data_by_key(f).map(|i| i.clone()))
                .collect::<Vec<Option<Tagged<Value>>>>()
        };
        if reverse {
            vec.sort_by_cached_key(|item| std::cmp::Reverse(calc_key(item)));
        } else {
            vec.sort_by_cached_key(calc_key);
        };

        for item in vec {
            yield item.into();
        }
    }))
}
