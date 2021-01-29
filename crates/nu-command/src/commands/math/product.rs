use crate::commands::math::reducers::{reducer_for, Reduce};
use crate::commands::math::utils::run_with_function;
use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, UntaggedValue, Value};

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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_with_function(
            RunnableContext {
                input: args.input,
                scope: args.scope.clone(),
                shell_manager: args.shell_manager,
                host: args.host,
                ctrl_c: args.ctrl_c,
                current_errors: args.current_errors,
                name: args.call_info.name_tag,
            },
            product,
        )
        .await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Get the product of a list of numbers",
            example: "echo [2 3 3 4] | math product",
            result: Some(vec![UntaggedValue::int(72).into()]),
        }]
    }
}

fn to_byte(value: &Value) -> Option<Value> {
    match &value.value {
        UntaggedValue::Primitive(Primitive::Int(num)) => {
            Some(UntaggedValue::Primitive(Primitive::Filesize(num.clone())).into_untagged_value())
        }
        _ => None,
    }
}

/// Calculate product of given values
pub fn product(values: &[Value], name: &Tag) -> Result<Value, ShellError> {
    let prod = reducer_for(Reduce::Product);

    let first = values.get(0).ok_or_else(|| {
        ShellError::unexpected("Cannot perform aggregate math operation on empty data")
    })?;

    match first {
        v if v.is_filesize() => to_byte(&prod(
            UntaggedValue::int(1).into_untagged_value(),
            values
                .iter()
                .map(|v| match v {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Filesize(num)),
                        ..
                    } => UntaggedValue::int(num.clone()).into_untagged_value(),
                    other => other.clone(),
                })
                .collect::<Vec<_>>(),
        )?)
        .ok_or_else(|| {
            ShellError::labeled_error(
                "could not convert to decimal",
                "could not convert to decimal",
                &name.span,
            )
        }),

        v if v.is_none() => prod(
            UntaggedValue::int(1).into_untagged_value(),
            values
                .iter()
                .map(|v| match v {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        ..
                    } => UntaggedValue::int(1).into_untagged_value(),
                    other => other.clone(),
                })
                .collect::<Vec<_>>(),
        ),
        _ => prod(UntaggedValue::int(1).into_untagged_value(), values.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::SubCommand;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(SubCommand {})?)
    }
}
