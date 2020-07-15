use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::ValueExt;

#[derive(Deserialize)]
struct Arguments {
    rest: Vec<ColumnPath>,
    #[serde(rename(deserialize = "char"))]
    char_: Option<Tagged<char>>,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str trim"
    }

    fn signature(&self) -> Signature {
        Signature::build("str trim")
            .rest(
                SyntaxShape::ColumnPath,
                "optionally trim text by column paths",
            )
            .named(
                "char",
                SyntaxShape::String,
                "character to trim (default: whitespace)",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "trims text"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        operate(args, registry).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Trim whitespace",
                example: "echo 'Nu shell ' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
            Example {
                description: "Trim a specific character",
                example: "echo '=== Nu shell ===' | str trim -c '=' | str trim",
                result: Some(vec![Value::from("Nu shell")]),
            },
        ]
    }
}

async fn operate(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();

    let (Arguments { rest, char_ }, input) = args.process(&registry).await?;

    let column_paths: Vec<_> = rest;
    let to_trim = char_.map(|tagged| tagged.item);

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, v.tag(), to_trim)?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let swapping = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, old.tag(), to_trim)),
                    );

                    match swapping {
                        Ok(new_value) => {
                            ret = new_value;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(input: &Value, tag: impl Into<Tag>, char_: Option<char>) -> Result<Value, ShellError> {
    match &input.value {
        UntaggedValue::Primitive(Primitive::Line(s))
        | UntaggedValue::Primitive(Primitive::String(s)) => {
            Ok(UntaggedValue::string(match char_ {
                None => String::from(s.trim()),
                Some(ch) => trim_char(s, ch, true, true),
            })
            .into_value(tag))
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

pub fn trim_char(from: &str, to_trim: char, leading: bool, trailing: bool) -> String {
    let mut trimmed = String::from("");
    let mut backlog = String::from("");
    let mut at_left = true;
    from.chars().for_each(|ch| match ch {
        c if c == to_trim => {
            if !(leading && at_left) {
                if trailing {
                    backlog.push(c)
                } else {
                    trimmed.push(c)
                }
            }
        }
        other => {
            at_left = false;
            if trailing {
                trimmed.push_str(backlog.as_str());
                backlog = String::from("");
            }
            trimmed.push(other);
        }
    });

    trimmed
}

#[cfg(test)]
mod tests {
    use super::{action, SubCommand};
    use nu_plugin::test_helpers::value::string;
    use nu_source::Tag;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn trims() {
        let word = string("andres ");
        let expected = string("andres");

        let actual = action(&word, Tag::unknown(), None).unwrap();
        assert_eq!(actual, expected);
    }
}
