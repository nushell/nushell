use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;

use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

pub struct SubCommand;

impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "math sum"
    }

    fn signature(&self) -> Signature {
        Signature::build("math sum")
    }

    fn usage(&self) -> &str {
        "Finds the sum of a list of numbers or tables"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_function(args, summation)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sum a list of numbers",
                example: "echo [1 2 3] | math sum",
                result: Some(vec![UntaggedValue::int(6).into()]),
            },
            Example {
                description: "Get the disk usage for the current directory",
                example: "ls --all --du | get size | math sum",
                result: None,
            },
        ]
    }
}

fn to_byte(value: &Value) -> Option<Value> {
    match &value.value {
        UntaggedValue::Primitive(Primitive::Int(num)) => {
            Some(UntaggedValue::Primitive(Primitive::Filesize(*num as u64)).into_untagged_value())
        }
        _ => None,
    }
}

pub fn summation(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    let first = values.get(0).ok_or_else(|| {
        ShellError::labeled_error(
            "Cannot perform aggregate math operation on empty data",
            "expected input",
            name.span,
        )
    })?;

    match first {
        v if v.is_filesize() => to_byte(&sum(
            UntaggedValue::int(0).into_untagged_value(),
            values
                .to_vec()
                .iter()
                .map(|v| match v {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Filesize(num)),
                        ..
                    } => UntaggedValue::int(*num as i64).into_untagged_value(),
                    other => other.clone(),
                })
                .collect::<Vec<_>>(),
        )?)
        .ok_or_else(|| {
            ShellError::labeled_error(
                "could not convert to big decimal",
                "could not convert to big decimal",
                &name.span,
            )
        }),
        v if v.is_duration() => sum(UntaggedValue::int(0).into_untagged_value(), values.to_vec()),
        // v is nothing primitive
        v if v.is_none() => sum(
            UntaggedValue::int(0).into_untagged_value(),
            values
                .to_vec()
                .iter()
                .map(|v| match v {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        ..
                    } => UntaggedValue::int(0).into_untagged_value(),
                    other => other.clone(),
                })
                .collect::<Vec<_>>(),
        ),
        _ => sum(UntaggedValue::int(0).into_untagged_value(), values.to_vec()),
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
