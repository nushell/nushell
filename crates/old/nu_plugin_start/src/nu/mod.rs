use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape};

use crate::start::Start;

impl Plugin for Start {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("start")
            .usage("Opens each file/directory/URL using the default application")
            .rest(
                "rest",
                SyntaxShape::String,
                "files/urls/directories to open",
            )
            .named(
                "application",
                SyntaxShape::String,
                "Specifies the application used for opening the files/directories/urls",
                Some('a'),
            )
            .filter())
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.parse(call_info)?;
        self.exec().map(|_| Vec::new())
    }
}
