use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::prelude::*;

pub struct FromSSV;

#[derive(Deserialize)]
pub struct FromSSVArgs {
    headerless: bool,
}

const STRING_REPRESENTATION: &str = "from-ssv";

impl WholeStreamCommand for FromSSV {
    fn name(&self) -> &str {
        STRING_REPRESENTATION
    }

    fn signature(&self) -> Signature {
        Signature::build(STRING_REPRESENTATION).switch("headerless")
    }

    fn usage(&self) -> &str {
        "Parse text as whitespace-separated values and create a table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, from_ssv)?.run()
    }
}

fn string_to_table(s: &str, headerless: bool) -> Vec<Vec<(String, String)>> {
    let mut lines = s.lines().filter(|l| !l.trim().is_empty());

    let headers = lines
        .next()
        .unwrap()
        .split_whitespace()
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();

    let header_row = if headerless {
        (1..=headers.len())
            .map(|i| format!("Column{}", i))
            .collect::<Vec<String>>()
    } else {
        headers
    };

    lines
        .map(|l| {
            header_row
                .iter()
                .zip(l.split_whitespace())
                .map(|(a, b)| (String::from(a), String::from(b)))
                .collect()
        })
        .collect()
}

fn from_ssv_string_to_value(
    s: &str,
    headerless: bool,
    tag: impl Into<Tag>,
) -> Option<Tagged<Value>> {
    let tag = tag.into();
    let rows = string_to_table(s, headerless)
        .iter()
        .map(|row| {
            let mut tagged_dict = TaggedDictBuilder::new(&tag);
            for (col, entry) in row {
                tagged_dict.insert_tagged(
                    col,
                    Value::Primitive(Primitive::String(String::from(entry))).tagged(&tag),
                )
            }
            tagged_dict.into_tagged_value()
        })
        .collect();

    Some(Value::Table(rows).tagged(&tag))
}

fn from_ssv(
    FromSSVArgs { headerless }: FromSSVArgs,
    RunnableContext { input, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;
        let mut concat_string = String::new();
        let mut latest_tag: Option<Tag> = None;

        for value in values {
            let value_tag = value.tag();
            latest_tag = Some(value_tag.clone());
            match value.item {
                Value::Primitive(Primitive::String(s)) => {
                    concat_string.push_str(&s);
                }
                _ => yield Err(ShellError::labeled_error_with_secondary (
                    "Expected a string from pipeline",
                    "requires string input",
                    &name,
                    "value originates from here",
                    &value_tag
                )),
            }
        }

        match from_ssv_string_to_value(&concat_string, headerless, name.clone()) {
            Some(x) => match x {
                Tagged { item: Value::Table(list), ..} => {
                    for l in list { yield ReturnSuccess::value(l) }
                }
                x => yield ReturnSuccess::value(x)
            },
            None => if let Some(tag) = latest_tag {
                yield Err(ShellError::labeled_error_with_secondary(
                    "Could not parse as SSV",
                    "input cannot be parsed ssv",
                    &name,
                    "value originates from here",
                    &tag,
                ))
            },
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owned(x: &str, y: &str) -> (String, String) {
        (String::from(x), String::from(y))
    }

    #[test]
    fn it_trims_empty_and_whitespace_only_lines() {
        let input = r#"

            a       b

            1    2

            3 4
        "#;
        let result = string_to_table(input, false);
        assert_eq!(
            result,
            vec![
                vec![owned("a", "1"), owned("b", "2")],
                vec![owned("a", "3"), owned("b", "4")]
            ]
        );
    }

    #[test]
    fn it_ignores_headers_when_headerless() {
        let input = r#"
            a b
            1 2
            3 4
        "#;
        let result = string_to_table(input, true);
        assert_eq!(
            result,
            vec![
                vec![owned("Column1", "1"), owned("Column2", "2")],
                vec![owned("Column1", "3"), owned("Column2", "4")]
            ]
        );
    }
}
