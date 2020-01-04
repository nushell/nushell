use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use crate::utils::data_processing::reduce;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
use num_traits::cast::ToPrimitive;

pub struct ReduceBy;

#[derive(Deserialize)]
pub struct ReduceByArgs {
    reduce_with: Option<Tagged<String>>,
}

impl WholeStreamCommand for ReduceBy {
    fn name(&self) -> &str {
        "reduce-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce-by").named(
            "reduce_with",
            SyntaxShape::String,
            "the command to reduce by with",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows reduced by the command given."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, reduce_by)?.run()
    }
}

pub fn reduce_by(
    ReduceByArgs { reduce_with }: ReduceByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Value> = input.values.collect().await;

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {

            let reduce_with = if let Some(reducer) = reduce_with {
                Some(reducer.item().clone())
            } else {
                None
            };

            match reduce(&values[0], reduce_with, name) {
                Ok(reduced) => yield ReturnSuccess::value(reduced),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}
