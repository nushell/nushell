use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, TaggedDictBuilder, UntaggedValue};

pub struct FromURL;

#[async_trait]
impl WholeStreamCommand for FromURL {
    fn name(&self) -> &str {
        "from url"
    }

    fn signature(&self) -> Signature {
        Signature::build("from url")
    }

    fn usage(&self) -> &str {
        "Parse url-encoded string as a table."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        from_url(args).await
    }
}

async fn from_url(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let tag = args.name_tag();
    let input = args.input;

    let concat_string = input.collect_string(tag.clone()).await?;

    let result = serde_urlencoded::from_str::<Vec<(String, String)>>(&concat_string.item);

    match result {
        Ok(result) => {
            let mut row = TaggedDictBuilder::new(tag);

            for (k, v) in result {
                row.insert_untagged(k, UntaggedValue::string(v));
            }

            Ok(OutputStream::one(ReturnSuccess::value(row.into_value())))
        }
        _ => Err(ShellError::labeled_error_with_secondary(
            "String not compatible with url-encoding",
            "input not url-encoded",
            tag,
            "value originates from here",
            concat_string.tag,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::FromURL;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(FromURL {})
    }
}
