use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};

use crate::start::Start;

impl Plugin for Start {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("start")
            .desc("Opens each file/directory/URL using the default application")
            .rest(SyntaxShape::String, "files/urls/directories to open")
            .named(
                "application",
                SyntaxShape::String,
                "Specifies the application used for opening the files/directories/urls",
                Some('a'),
            ))
    }
    fn sink(&mut self, call_info: CallInfo, input: Vec<Value>) {
        self.parse(call_info, input);
        if let Err(e) = self.exec() {
            println!("{}", e);
        }
    }
}
