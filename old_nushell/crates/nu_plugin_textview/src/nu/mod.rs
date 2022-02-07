use crate::textview::{view_text_value, TextView};
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Signature, Value};

impl Plugin for TextView {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("textview").desc("Autoview of text data."))
    }

    fn sink(&mut self, _call_info: CallInfo, input: Vec<Value>) {
        if !input.is_empty() {
            view_text_value(&input[0]);
        }
    }
}
