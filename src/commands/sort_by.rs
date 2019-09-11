use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

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
        Signature::build("sort-by").rest(SyntaxType::String)
    }

    fn usage(&self) -> &str {
        "Sort by the given columns."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, sort_by)?.run()
    }
}

fn sort_by(
    SortByArgs { rest }: SortByArgs,
    mut context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    Ok(OutputStream::new(async_stream_block! {
        let mut vec = context.input.drain_vec().await;

        let calc_key = |item: &Tagged<Value>| {
            rest.iter()
                .map(|f| item.get_data_by_key(f).map(|i| i.clone()))
                .collect::<Vec<Option<Tagged<Value>>>>()
        };
        vec.sort_by_cached_key(calc_key);

        for item in vec {
            yield item.into();
        }
    }))
}
