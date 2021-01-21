use crate::examples::sample::ls::file_listing;

use nu_engine::{CommandArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};
use nu_stream::{OutputStream, ToOutputStream};

use async_trait::async_trait;
use futures::StreamExt;

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "ls"
    }

    fn signature(&self) -> Signature {
        Signature::build("ls")
    }

    fn usage(&self) -> &str {
        "Mock ls."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();

        let mut base_value =
            UntaggedValue::string("Andrés N. Robalino in Portland").into_value(name_tag);
        let input: Vec<Value> = args.input.collect().await;

        if let Some(first) = input.get(0) {
            base_value = first.clone()
        }

        Ok(futures::stream::iter(
            file_listing()
                .iter()
                .map(|row| Value {
                    value: row.value.clone(),
                    tag: base_value.tag.clone(),
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(ReturnSuccess::value),
        )
        .to_output_stream())
    }
}
