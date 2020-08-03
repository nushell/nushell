use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;

use nu_protocol::{
    hir::{convert_number_to_u64, Number},
    Primitive, Signature, UntaggedValue, Value,
};

pub struct SubCommand;

#[async_trait]
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
            summation,
        )
        .await
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
        UntaggedValue::Primitive(Primitive::Int(num)) => Some(
            UntaggedValue::Primitive(Primitive::Filesize(convert_number_to_u64(&Number::Int(
                num.clone(),
            ))))
            .into_untagged_value(),
        ),
        _ => None,
    }
}

pub fn summation(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let sum = reducer_for(Reduce::Summation);

    let first = values.get(0).ok_or_else(|| {
        ShellError::unexpected("Cannot perform aggregate math operation on empty data")
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
                    } => UntaggedValue::int(*num as usize).into_untagged_value(),
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
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }
}
