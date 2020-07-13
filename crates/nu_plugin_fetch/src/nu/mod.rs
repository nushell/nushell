use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape};

use crate::fetch::fetch;
use crate::Fetch;

impl Plugin for Fetch {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("fetch")
            .desc("Load from a URL into a cell, convert to table if possible (avoid by appending '--raw')")
            .required(
                "URL",
                SyntaxShape::String,
                "the URL to fetch the contents from",
            )
            .named(
                "user",
                SyntaxShape::Any,
                "the username when authenticating",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::Any,
                "the password when authenticating",
                Some('p'),
            )
            .switch("raw", "fetch contents as text rather than a table", Some('r'))
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(callinfo)?;
        Ok(vec![block_on(fetch(
            &self.path.clone().ok_or_else(|| {
                ShellError::labeled_error("internal error: path not set", "path not set", &self.tag)
            })?,
            self.has_raw,
            self.user.clone(),
            self.password.clone(),
        ))])
    }
}
