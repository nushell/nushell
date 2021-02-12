use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_value_ext::ValueExt;

use bigdecimal::BigDecimal;
use std::str::FromStr;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str to-decimal"
    }

    fn signature(&self) -> Signature {
        Signature::build("str to-decimal").rest(
            SyntaxShape::ColumnPath,
            "optionally convert text into decimal by column paths",
        )
    }

    fn usage(&self) -> &str {
        "converts text into decimal"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert to decimal",
            example: "echo '3.1415' | str to-decimal",
            result: None,
        }]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rest }, input) = args.process().await?;

    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let other = s.trim();
            let out = match BigDecimal::from_str(other) {
                Ok(v) => UntaggedValue::decimal(v),
                Err(reason) => {
                    return Err(ShellError::labeled_error(
                        "could not parse as decimal",
                        reason.to_string(),
                        tag.into().span,
                    ))
                }
            };
            Ok(out.into_value(tag))
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                tag.into().span,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{action, SubCommand};
    use nu_source::Tag;
    use nu_test_support::value::{decimal_from_float, string};

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn turns_to_integer() {
        let word = string("3.1415");
        let expected = decimal_from_float(3.1415);

        let actual = action(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_decimallike_string() {
        let decimal_str = string("11.6anra");

        let actual = action(&decimal_str, Tag::unknown());

        assert!(actual.is_err());
    }
}
