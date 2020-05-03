// implement the Plugin trait here

use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};

use crate::start::Start;

impl Plugin for Start {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("start")
            .desc("Open an application")
            .optional("path/url/app", SyntaxShape::String, "app to open")
            .named(
                "application",
                SyntaxShape::String,
                "application to open the file in",
                Some('a'),
            ))
    }

    fn sink(&mut self, call_info: CallInfo, _input: Vec<Value>) {
        let args = call_info.args;

        let path = match args.nth(0).map(|s| s.as_string()) {
            Some(Ok(s)) => s,
            _ => String::new(),
        };

        let res = if let Some(Ok(application)) = args.get("application").map(|s| s.as_string()) {
            open::with(path, application)
        } else {
            open::that(path)
        };

        if res.is_err() {
            println!("Failed to open the resource");
        }
    }
}
