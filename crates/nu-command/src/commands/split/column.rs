use crate::prelude::*;
use log::trace;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::Tagged;

#[derive(Deserialize)]
struct SplitColumnArgs {
    separator: Tagged<String>,
    rest: Vec<Tagged<String>>,
    #[serde(rename(deserialize = "collapse-empty"))]
    collapse_empty: bool,
}

pub struct SubCommand;

#[async_trait]
impl WholeStreamCommand for SubCommand {
    fn name(&self) -> &str {
        "split column"
    }

    fn signature(&self) -> Signature {
        Signature::build("split column")
            .required(
                "separator",
                SyntaxShape::Any,
                "the character that denotes what separates columns",
            )
            .switch("collapse-empty", "remove empty columns", Some('c'))
            .rest(SyntaxShape::String, "column names to give the new columns")
    }

    fn usage(&self) -> &str {
        "splits contents across multiple columns via the separator."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        split_column(args).await
    }
}

async fn split_column(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_span = args.call_info.name_tag.span;
    let (
        SplitColumnArgs {
            separator,
            rest,
            collapse_empty,
        },
        input,
    ) = args.process().await?;

    Ok(input
        .map(move |v| {
            if let Ok(s) = v.as_string() {
                let splitter = separator.replace("\\n", "\n");
                trace!("splitting with {:?}", splitter);

                let split_result: Vec<_> = if collapse_empty {
                    s.split(&splitter).filter(|s| !s.is_empty()).collect()
                } else {
                    s.split(&splitter).collect()
                };

                trace!("split result = {:?}", split_result);

                let positional: Vec<_> = rest.iter().map(|f| f.item.clone()).collect();

                // If they didn't provide column names, make up our own
                if positional.is_empty() {
                    let mut gen_columns = vec![];
                    for i in 0..split_result.len() {
                        gen_columns.push(format!("Column{}", i + 1));
                    }

                    let mut dict = TaggedDictBuilder::new(&v.tag);
                    for (&k, v) in split_result.iter().zip(gen_columns.iter()) {
                        dict.insert_untagged(v.clone(), Primitive::String(k.into()));
                    }

                    ReturnSuccess::value(dict.into_value())
                } else {
                    let mut dict = TaggedDictBuilder::new(&v.tag);
                    for (&k, v) in split_result.iter().zip(positional.iter()) {
                        dict.insert_untagged(
                            v,
                            UntaggedValue::Primitive(Primitive::String(k.into())),
                        );
                    }
                    ReturnSuccess::value(dict.into_value())
                }
            } else {
                Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    name_span,
                    "value originates from here",
                    v.tag.span,
                ))
            }
        })
        .to_output_stream())
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
