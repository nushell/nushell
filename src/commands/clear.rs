use crate::commands::WholeStreamCommand;
use nu_protocol::Signature;
pub struct Clear;

impl WholeStreamCommand for Clear {
    fn name(&self) -> &str {
        "clear"
    }
    fn signature(&self) -> Signature {
        Signature::build("clear")
    }
    fn usage(&self) -> &str {
        "clears the terminal"
    }
}
