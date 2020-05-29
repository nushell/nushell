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

#[async_trait]
impl WholeStreamCommand for ReduceBy {
    fn name(&self) -> &str {
        "reduce-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce-by").named(
            "reduce_with",
            SyntaxShape::String,
            "the command to reduce by with",
            Some('w'),
        )
    }

    fn usage(&self) -> &str {
        "Creates a new table with the data from the tables rows reduced by the command given."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        reduce_by(args, registry)
    }
}

pub fn reduce_by(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name = args.call_info.name_tag.clone();
    let stream = async_stream! {
        let (ReduceByArgs { reduce_with }, mut input) = args.process(&registry).await?;
        let values: Vec<Value> = input.collect().await;

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

#[cfg(test)]
mod tests {
    use super::ReduceBy;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ReduceBy {})
    }
}
