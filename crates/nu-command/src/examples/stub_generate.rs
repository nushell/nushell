use nu_engine::{CommandArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tag};
use nu_stream::OutputStream;

use async_trait::async_trait;
use serde::Deserialize;

pub struct Command;

#[derive(Deserialize)]
struct Arguments {
    path: Option<bool>,
}

#[async_trait]
impl WholeStreamCommand for Command {
    fn name(&self) -> &str {
        "stub open"
    }

    fn signature(&self) -> Signature {
        Signature::build("stub open").switch("path", "Add a mocked path", Some('p'))
    }

    fn usage(&self) -> &str {
        "Generates tables and metadata that mimics behavior of real commands in controlled ways."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();

        let (Arguments { path: mocked_path }, _input) = args.process().await?;

        let out = UntaggedValue::string("Yehuda Katz in Ecuador");

        if let Some(true) = mocked_path {
            Ok(OutputStream::one(Ok(ReturnSuccess::Value(Value {
                value: out,
                tag: Tag {
                    anchor: Some(mock_path()),
                    span: name_tag.span,
                },
            }))))
        } else {
            Ok(OutputStream::one(Ok(ReturnSuccess::Value(
                out.into_value(name_tag),
            ))))
        }
    }
}

pub fn mock_path() -> AnchorLocation {
    let path = String::from("path/to/las_best_arepas_in_the_world.txt");

    AnchorLocation::File(path)
}
