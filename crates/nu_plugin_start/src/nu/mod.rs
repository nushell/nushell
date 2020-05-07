use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape, Value};

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
        self.parse(call_info);
        println!("{:?}", input);
        input.iter().for_each(|val| {
            self.add_filename(val);
        });
    }
}
