use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
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
    Ok(OutputStream::new(async_stream! {
        let mut vec = context.input.drain_vec().await;

        let calc_key = |item: &Value| {
            rest.iter()
                .map(|f| get_data_by_key(item, f.borrow_spanned()).map(|i| i.clone()))
                .collect::<Vec<Option<Value>>>()
        };
        vec.sort_by_cached_key(calc_key);

        for item in vec {
            yield item.into();
        }
    }))
}
