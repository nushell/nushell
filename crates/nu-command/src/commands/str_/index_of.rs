use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::ShellTypeName;
use nu_protocol::{
    ColumnPath, Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
use nu_value_ext::{as_string, ValueExt};

#[derive(Deserialize)]
struct Arguments {
    pattern: Tagged<String>,
    rest: Vec<ColumnPath>,
    range: Option<Value>,
    end: bool,
}

pub struct SubCommand;

#[derive(Clone)]
pub struct IndexOfOptionalBounds(i32, i32);

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "str index-of"
    }

    fn signature(&self) -> Signature {
        Signature::build("str index-of")
            .required(
                "pattern",
                SyntaxShape::String,
                "the pattern to find index of",
            )
            .rest(
                SyntaxShape::ColumnPath,
                "optionally returns index of pattern in string by column paths",
            )
            .named(
                "range",
                SyntaxShape::Any,
                "optional start and/or end index",
                Some('r'),
            )
            .switch("end", "search from the end of the string", Some('e'))
    }

    fn usage(&self) -> &str {
        "Returns starting index of given pattern in string counting from 0. Returns -1 when there are no results."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        operate(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns index of pattern in string",
                example: "echo 'my_library.rb' | str index-of '.rb'",
                result: Some(vec![UntaggedValue::int(10).into_untagged_value()]),
            },
            Example {
                description: "Returns index of pattern in string with start index",
                example: "echo '.rb.rb' | str index-of '.rb' -r '1,'",
                result: Some(vec![UntaggedValue::int(3).into_untagged_value()]),
            },
            Example {
                description: "Returns index of pattern in string with end index",
                example: "echo '123456' | str index-of '6' -r ',4'",
                result: Some(vec![UntaggedValue::int(-1).into_untagged_value()]),
            },
            Example {
                description: "Returns index of pattern in string with start and end index",
                example: "echo '123456' | str index-of '3' -r '1,4'",
                result: Some(vec![UntaggedValue::int(2).into_untagged_value()]),
            },
            Example {
                description: "Alternatively you can use this form",
                example: "echo '123456' | str index-of '3' -r [1 4]",
                result: Some(vec![UntaggedValue::int(2).into_untagged_value()]),
            },
            Example {
                description: "Returns index of pattern in string",
                example: "echo '/this/is/some/path/file.txt' | str index-of '/' -e",
                result: Some(vec![UntaggedValue::int(18).into_untagged_value()]),
            },
        ]
    }
}

async fn operate(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        Arguments {
            pattern,
            rest,
            range,
            end,
        },
        input,
    ) = args.process().await?;
    let range = range.unwrap_or_else(|| {
        UntaggedValue::Primitive(Primitive::String("".to_string())).into_untagged_value()
    });
    let column_paths: Vec<_> = rest;

    Ok(input
        .map(move |v| {
            if column_paths.is_empty() {
                ReturnSuccess::value(action(&v, &pattern, &range, end, v.tag())?)
            } else {
                let mut ret = v;

                for path in &column_paths {
                    let range = range.clone();
                    let pattern = pattern.clone();
                    ret = ret.swap_data_by_column_path(
                        path,
                        Box::new(move |old| action(old, &pattern, &range, end, old.tag())),
                    )?;
                }

                ReturnSuccess::value(ret)
            }
        })
        .to_output_stream())
}

fn action(
    input: &Value,
    pattern: &str,
    range: &Value,
    end: bool,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let r = process_range(&input, &range)?;
    match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let start_index = r.0 as usize;
            let end_index = r.1 as usize;

            if end {
                if let Some(result) = s[start_index..end_index].rfind(pattern) {
                    Ok(UntaggedValue::int(result + start_index).into_value(tag))
                } else {
                    let not_found = -1;
                    Ok(UntaggedValue::int(not_found).into_value(tag))
                }
            } else if let Some(result) = s[start_index..end_index].find(pattern) {
                Ok(UntaggedValue::int(result + start_index).into_value(tag))
            } else {
                let not_found = -1;
                Ok(UntaggedValue::int(not_found).into_value(tag))
            }
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

fn process_range(input: &Value, range: &Value) -> Result<IndexOfOptionalBounds, ShellError> {
    let input_len = match &input.value {
        UntaggedValue::Primitive(Primitive::String(s)) => s.len(),
        _ => 0,
    };
    let min_index_str = String::from("0");
    let max_index_str = input_len.to_string();
    let r = match &range.value {
        UntaggedValue::Primitive(Primitive::String(s)) => {
            let indexes: Vec<&str> = s.split(',').collect();

            let start_index = indexes.get(0).unwrap_or(&&min_index_str[..]).to_string();

            let end_index = indexes.get(1).unwrap_or(&&max_index_str[..]).to_string();

            Ok((start_index, end_index))
        }
        UntaggedValue::Table(indexes) => {
            if indexes.len() > 2 {
                Err(ShellError::labeled_error(
                    "there shouldn't be more than two indexes",
                    "too many indexes",
                    range.tag(),
                ))
            } else {
                let idx: Vec<String> = indexes
                    .iter()
                    .map(|v| as_string(v).unwrap_or_else(|_| String::from("")))
                    .collect();

                let start_index = idx.get(0).unwrap_or(&min_index_str).to_string();
                let end_index = idx.get(1).unwrap_or(&max_index_str).to_string();

                Ok((start_index, end_index))
            }
        }
        other => {
            let got = format!("got {}", other.type_name());
            Err(ShellError::labeled_error(
                "value is not string",
                got,
                range.tag(),
            ))
        }
    }?;

    let start_index = r.0.parse::<i32>().unwrap_or(0);
    let end_index = r.1.parse::<i32>().unwrap_or(input_len as i32);

    if start_index < 0 || start_index > end_index {
        return Err(ShellError::labeled_error(
            "start index can't be negative or greater than end index",
            "Invalid start index",
            range.tag(),
        ));
    }

    if end_index < 0 || end_index < start_index || end_index > input_len as i32 {
        return Err(ShellError::labeled_error(
            "end index can't be negative, smaller than start index or greater than input length",
            "Invalid end index",
            range.tag(),
        ));
    }
    Ok(IndexOfOptionalBounds(start_index, end_index))
}
#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::{action, SubCommand};
    use nu_protocol::{Primitive, UntaggedValue};
    use nu_source::Tag;
    use nu_test_support::value::string;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(SubCommand {})
    }

    #[test]
    fn returns_index_of_substring() {
        let word = string("Cargo.tomL");
        let pattern = ".tomL";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String("".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int(5.into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn index_of_does_not_exist_in_string() {
        let word = string("Cargo.tomL");
        let pattern = "Lm";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String("".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int((-1).into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn returns_index_of_next_substring() {
        let word = string("Cargo.Cargo");
        let pattern = "Cargo";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String("1,".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int(6.into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
    #[test]
    fn index_does_not_exist_due_to_end_index() {
        let word = string("Cargo.Banana");
        let pattern = "Banana";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String(",5".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int((-1).into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn returns_index_of_nums_in_middle_due_to_index_limit_from_both_ends() {
        let word = string("123123123");
        let pattern = "123";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String("2,6".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int(3.into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn index_does_not_exists_due_to_strict_bounds() {
        let word = string("123456");
        let pattern = "1";
        let end = false;
        let index_of_bounds =
            UntaggedValue::Primitive(Primitive::String("2,4".to_string())).into_untagged_value();
        let expected = UntaggedValue::Primitive(Primitive::Int((-1).into())).into_untagged_value();

        let actual = action(&word, &pattern, &index_of_bounds, end, Tag::unknown()).unwrap();
        assert_eq!(actual, expected);
    }
}
