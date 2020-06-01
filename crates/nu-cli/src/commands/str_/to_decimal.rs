use crate::commands::WholeStreamCommand;
use crate::prelude::*;
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert to decimal",
            example: "echo '3.1415' | str to-decimal",
            result: None,
        }]
    }
}

fn operate(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let stream = async_stream! {
        let (Arguments { rest }, mut input) = args.process(&registry).await?;

        let column_paths: Vec<_> = rest.iter().map(|x| x.clone()).collect();

        while let Some(v) = input.next().await {
            if column_paths.is_empty() {
                match action(&v, v.tag()) {
                    Ok(out) => yield ReturnSuccess::value(out),
                    Err(err) => {
                        yield Err(err);
                        return;
                    }
                }
            } else {

                let mut ret = v.clone();

                for path in &column_paths {
                    let swapping = ret.swap_data_by_column_path(path, Box::new(move |old| action(old, old.tag())));

                    match swapping {
                        Ok(new_value) => {
                            ret = new_value;
                        }
                        Err(err) => {
                            yield Err(err);
                            return;
                        }
                    }
                }

                yield ReturnSuccess::value(ret);
            }
        }
    };

    Ok(stream.to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            let other = s.trim();
            let out = match BigDecimal::from_str(other) {
                Ok(v) => UntaggedValue::decimal(v),
                Err(_) => UntaggedValue::string(s),
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
    use super::{action, SubCommand};
    use nu_plugin::test_helpers::value::{decimal, string};
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn turns_to_integer() {
        let word = string("3.1415");
        let expected = decimal(3.1415);

        let actual = action(&word, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
