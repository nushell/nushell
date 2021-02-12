use super::variance::compute_variance as variance;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::Tagged;
use std::str::FromStr;

pub struct SubCommand;

#[derive(Deserialize)]
struct Arguments {
    sample: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math stddev"
    }

    fn signature(&self) -> Signature {
        Signature::build("math stddev").switch(
            "sample",
            "calculate sample standard deviation",
            Some('s'),
        )
    }

    fn usage(&self) -> &str {
        "Finds the stddev of a list of numbers or tables"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name = args.call_info.name_tag.clone();
        let (Arguments { sample }, mut input) = args.process().await?;

        let values: Vec<Value> = input.drain_vec().await;

        let n = if let Tagged { item: true, .. } = sample {
            values.len() - 1
        } else {
            values.len()
        };

        let res = if values.iter().all(|v| v.is_primitive()) {
            compute_stddev(&values, n, &name)
        } else {
            // If we are not dealing with Primitives, then perhaps we are dealing with a table
            // Create a key for each column name
            let mut column_values = IndexMap::new();
            for value in values {
                if let UntaggedValue::Row(row_dict) = &value.value {
                    for (key, value) in row_dict.entries.iter() {
                        column_values
                            .entry(key.clone())
                            .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                            .or_insert(vec![value.clone()]);
                    }
                }
            }
            // The mathematical function operates over the columns of the table
            let mut column_totals = IndexMap::new();
            for (col_name, col_vals) in column_values {
                if let Ok(out) = compute_stddev(&col_vals, n, &name) {
                    column_totals.insert(col_name, out);
                }
            }

            if column_totals.keys().len() == 0 {
                return Err(ShellError::labeled_error(
                    "Attempted to compute values that can't be operated on",
                    "value appears here",
                    name.span,
                ));
            }

            Ok(UntaggedValue::Row(Dictionary {
                entries: column_totals,
            })
            .into_untagged_value())
        }?;

        if res.value.is_table() {
            Ok(OutputStream::from(
                res.table_entries()
                    .map(|v| ReturnSuccess::value(v.clone()))
                    .collect::<Vec<_>>(),
            ))
        } else {
            Ok(OutputStream::one(ReturnSuccess::value(res)))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get the stddev of a list of numbers",
                example: "echo [1 2 3 4 5] | math stddev",
                result: Some(vec![UntaggedValue::decimal(BigDecimal::from_str("1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573").expect("Could not convert to decimal from string")).into()]),
            },
            Example {
                description: "Get the sample stddev of a list of numbers",
                example: "echo [1 2 3 4 5] | math stddev -s",
                result: Some(vec![UntaggedValue::decimal(BigDecimal::from_str("1.581138830084189665999446772216359266859777569662608413428752426396297219319619110672124054189650148").expect("Could not convert to decimal from string")).into()]),
            },
        ]
    }
}

#[cfg(test)]
pub fn stddev(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    compute_stddev(values, values.len(), name)
}

pub fn compute_stddev(values: &[Value], n: usize, name: &Tag) -> Result<Value, ShellError> {
    let variance = variance(values, n, name)?.as_primitive()?;
    let sqrt_var = match variance {
        Primitive::Decimal(var) => var.sqrt(),
        _ => {
            return Err(ShellError::labeled_error(
                "Could not take square root of variance",
                "error occurred here",
                name.span,
            ))
        }
    };
    match sqrt_var {
        Some(stddev) => Ok(UntaggedValue::from(Primitive::Decimal(stddev)).into_value(name)),
        None => Err(ShellError::labeled_error(
            "Could not calculate stddev",
            "error occurred here",
            name.span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
