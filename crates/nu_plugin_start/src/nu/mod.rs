use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Signature, SyntaxShape, Value};
use std::process::{Command, Stdio};

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
            )
            .switch(
                "wait",
                "Blocks until used applications are closed",
                Some('w'),
            )
            .switch(
                "background",
                "Does not bring the application to the foreground",
                Some('b'),
            ))
    }

    fn sink(&mut self, call_info: CallInfo, _input: Vec<Value>) {
        let filenames = match call_info.args.positional {
            Some(values) => values
                .iter()
                .map(|val| val.as_string())
                .collect::<Result<Vec<String>, ShellError>>()
                .unwrap_or(vec![]),
            None => vec![],
        };

        Command::new("open")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .args(&filenames)
            .spawn()
            .unwrap();
    }
}
