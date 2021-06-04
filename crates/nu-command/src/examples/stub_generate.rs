use nu_engine::{CommandArgs, WholeStreamCommand};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tag};
use nu_stream::ActionStream;
pub struct Command;

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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let name_tag = args.call_info.name_tag.clone();

        let mocked_path = args.call_info.switch_present("path");

        let out = UntaggedValue::string("Yehuda Katz in Ecuador");

        if mocked_path {
            Ok(ActionStream::one(Ok(ReturnSuccess::Value(Value {
                value: out,
                tag: Tag {
                    anchor: Some(mock_path()),
                    span: name_tag.span,
                },
            }))))
        } else {
            Ok(ActionStream::one(Ok(ReturnSuccess::Value(
                out.into_value(name_tag),
            ))))
        }
    }
}

pub fn mock_path() -> AnchorLocation {
    let path = String::from("path/to/las_best_arepas_in_the_world.txt");

    AnchorLocation::File(path)
}
