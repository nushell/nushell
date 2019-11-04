use crate::commands::WholeStreamCommand;
use crate::data::TaggedDictBuilder;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry;
use crate::data::base::Block;
use crate::prelude::*;

use log::trace;

pub struct ReduceBy;

#[derive(Deserialize)]
pub struct ReduceByArgs {
    calculator: Block,
}

impl WholeStreamCommand for ReduceBy {
    fn name(&self) -> &str {
        "reduce-by"
    }

    fn signature(&self) -> Signature {
        Signature::build("reduce-by").required(
            "calculator",
            SyntaxShape::Block,
            "The block used for calculating values",
        )
    }

    fn usage(&self) -> &str {
        "Crates a new table with the data from the table rows reduced by the block given."
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
    ReduceByArgs { calculator }: ReduceByArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        trace!("{:?}", &calculator);

        if values.is_empty() {
            yield Err(ShellError::labeled_error(
                    "Expected table from pipeline",
                    "requires a table input",
                    name
                ))
        } else {
            match reduce(values, &calculator, name) {
                Ok(reduced) => yield ReturnSuccess::value(reduced),
                Err(err) => yield Err(err)
            }
        }
    };

    Ok(stream.to_output_stream())
}

pub fn reduce(
    values: Vec<Tagged<Value>>,
    calculator: &Block,
    tag: impl Into<Tag>,
) -> Result<Tagged<Value>, ShellError> {
    let tag = tag.into();

    let mut out = TaggedDictBuilder::new(&tag);

    Ok(out.into_tagged_value())
}

#[cfg(test)]
mod tests {

    use crate::commands::reduce_by::reduce;
    use crate::data::meta::*;
    use crate::Value;
    use indexmap::IndexMap;

    fn string(input: impl Into<String>) -> Tagged<Value> {
        Value::string(input.into()).tagged_unknown()
    }

    fn row(entries: IndexMap<String, Tagged<Value>>) -> Tagged<Value> {
        Value::row(entries).tagged_unknown()
    }

    fn table(list: &Vec<Tagged<Value>>) -> Tagged<Value> {
        Value::table(list).tagged_unknown()
    }
}
