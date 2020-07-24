use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Signature, UntaggedValue, Value};
use num_traits::identities::One;

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math product"
    }

    fn signature(&self) -> Signature {
        Signature::build("math product")
    }

    fn usage(&self) -> &str {
        "Finds the product of a list of numbers or tables"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_with_function(
            RunnableContext {
                input: args.input,
                registry: registry.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
                raw_input: args.raw_input,
            },
            product,
        )
        .await
    }
}

/// Calculate product of given values
pub fn product(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let prod = reducer_for(Reduce::Product);

    if values.iter().all(|v| v.is_primitive()) {
        Ok(prod(Value::one(), values.to_vec())?)
    } else {
        let mut column_values = IndexMap::new();

        for value in values {
            if let UntaggedValue::Row(row_dict) = value.value.clone() {
                for (key, value) in row_dict.entries.iter() {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert(vec![value.clone()]);
                }
            };
        }

        let mut column_product = IndexMap::new();

        for (col_name, col_vals) in column_values {
            let prod = prod(Value::one(), col_vals)?;

            column_product.insert(col_name, prod);
        }

        Ok(UntaggedValue::Row(Dictionary {
            entries: column_product,
        })
        .into_value(name))
    }
}
