use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::data::base::select_fields;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tagged;

#[derive(Deserialize)]
struct UniqArgs {
    rest: Vec<Tagged<String>>,
}

pub struct Uniq;

impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
            .rest(SyntaxShape::Any, "The columns to be unique over")
    }

    fn usage(&self) -> &str {
        "Return the unique rows"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, uniq)?.run()
    }
}

fn uniq(
    UniqArgs { rest: fields }: UniqArgs,
    RunnableContext { input, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {

    let fields: Vec<_> = fields.iter().map(|field| field.item.clone()).collect();
    let values: Vec<Value> = input.values.drain_vec().await;


    let objects = input
        .values
        .map(move |value| {
            // dbg!(&value);
            select_fields(&value, &fields, value.tag.clone())
        });

    /*
        for each row,
            check every column against every other row and column
            break out as soon as they are not equal
    */

    Ok(objects.to_output_stream())
}

