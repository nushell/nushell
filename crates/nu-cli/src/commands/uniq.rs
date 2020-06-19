use crate::commands::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use indexmap::map::IndexMap;
use nu_errors::ShellError;
use nu_protocol::Signature;

use num_bigint::ToBigUint;

pub struct Uniq;

#[async_trait]
impl WholeStreamCommand for Uniq {
    fn name(&self) -> &str {
        "uniq"
    }

    fn signature(&self) -> Signature {
        Signature::build("uniq")
    }

    fn usage(&self) -> &str {
        "Return the unique rows"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        uniq(args, registry).await
    }
}

async fn uniq(args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let input = args.input;
    let uniq_values = {
        let mut counter = IndexMap::<nu_protocol::Value, usize>::new();
        for line in input.into_vec().await {
            // TODO: Is there a way to collect and await at the end of the loop? (input.map() failed)
            *counter.entry(line).or_insert(0) += 1;
        }
        counter
    };

    let mut values_vec_deque = VecDeque::new();

    for item in uniq_values {
        use nu_protocol::{UntaggedValue, Value};
        let value = match item.0.value {
            UntaggedValue::Row(mut row) => {
                row.entries.insert(
                    "count".to_string(),
                    UntaggedValue::int(item.1.to_biguint().unwrap()).into_untagged_value(),
                );
                Value {
                    value: UntaggedValue::Row(row),
                    tag: item.0.tag,
                }
            }
            _ => panic!("Could not match: {:#?}", item),
        };
        values_vec_deque.push_back(value);
    }

    Ok(futures::stream::iter(values_vec_deque).to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::Uniq;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Uniq {})
    }
}
