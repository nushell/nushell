use futures::executor::block_on;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnValue, Signature, SyntaxShape, Value};

use crate::fetch::fetch_helper;
use crate::Fetch;

impl Plugin for Fetch {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("fetch")
            .desc("Load from a URL into a cell, convert to table if possible (avoid by appending '--raw')")
            .required(
                "path",
                SyntaxShape::Path,
                "the URL to fetch the contents from",
            )
            .switch("raw", "fetch contents as text rather than a table", Some('r'))
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.setup(callinfo)?;
        Ok(vec![])
    }

    fn filter(&mut self, value: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![block_on(fetch_helper(
            &self.path.clone().ok_or_else(|| {
                ShellError::labeled_error(
                    "internal error: path not set",
                    "path not set",
                    &value.tag,
                )
            })?,
            self.has_raw,
            value,
        ))])
    }
}
