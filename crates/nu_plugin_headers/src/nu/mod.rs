use crate::headers::Headers;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnSuccess, ReturnValue, Signature, Value};

impl Plugin for Headers {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("headers")
           .desc("Use the first row of the table as headers")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}