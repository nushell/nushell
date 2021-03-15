use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

use num_bigint::BigInt;
use num_traits::Num;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
    radix: Option<Tagged<u32>>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str to-int"
    }

    fn signature(&self) -> Signature {
        Signature::build("str to-int")
            .named("radix", SyntaxShape::Number, "radix of integer", Some('r'))
            .rest(
                SyntaxShape::ColumnPath,
                "optionally convert text into integer by column paths",
            )
    }

    fn usage(&self) -> &str {
        "converts text into integer"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert to an integer",
                example: "echo '255' | str to-int",
                result: Some(vec![UntaggedValue::int(255).into()]),
            },
            Example {
                description: "Convert str column to an integer",
                example: "echo [['count']; ['255']] | str to-int count | get count",
                result: Some(vec![UntaggedValue::int(255).into()]),
            },
            Example {
                description: "Convert to integer from binary",
                example: "echo '1101' | str to-int -r 2",
                result: Some(vec![UntaggedValue::int(13).into()]),
            },
            Example {
                description: "Convert to integer from hex",
                example: "echo 'FF' | str to-int -r 16",
                result: Some(vec![UntaggedValue::int(255).into()]),
            },
        ]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (Arguments { rest, radix }, input) = args.process().await?;

    let radix = radix.map(|r| r.item).unwrap_or(10);

    let column_paths: Vec<ColumnPath> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag(), radix)?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag(), radix)),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>, radix: u32) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let trimmed = s.trim();

            let out = match trimmed {
                b if b.starts_with("0b") => {
                    let num = match BigInt::from_str_radix(b.trim_start_matches("0b"), 2) {
                        Ok(n) => n,
                        Err(reason) => {
                            return Err(ShellError::labeled_error(
                                "could not parse as integer",
                                reason.to_string(),
                                tag.into().span,
                            ))
                        }
                    };
                    UntaggedValue::int(num)
                }
                h if h.starts_with("0x") => {
                    let num = match BigInt::from_str_radix(h.trim_start_matches("0x"), 16) {
                        Ok(n) => n,
                        Err(reason) => {
                            return Err(ShellError::labeled_error(
                                "could not parse as int",
                                reason.to_string(),
                                tag.into().span,
                            ))
                        }
                    };
                    UntaggedValue::int(num)
                }
                _ => {
                    let num = match BigInt::from_str_radix(trimmed, radix) {
                        Ok(n) => n,
                        Err(reason) => {
                            return Err(ShellError::labeled_error(
                                "could not parse as int",
                                reason.to_string(),
                                tag.into().span,
                            ))
                        }
                    };
                    UntaggedValue::int(num)
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
    use nu_test_support::value::{int, string};

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn turns_to_integer() {
        let word = string("10");
        let expected = int(10);

        let actual = action(&word, Tag::unknown(), 10).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn turns_binary_to_integer() {
        let s = string("0b101");
        let actual = action(&s, Tag::unknown(), 10).unwrap();
        assert_eq!(actual, int(5));
    }

    #[test]
    fn turns_hex_to_integer() {
        let s = string("0xFF");
        let actual = action(&s, Tag::unknown(), 16).unwrap();
        assert_eq!(actual, int(255));
    }

    #[test]
    fn communicates_parsing_error_given_an_invalid_integerlike_string() {
        let integer_str = string("36anra");

        let actual = action(&integer_str, Tag::unknown(), 10);

        assert!(actual.is_err());
    }
}
