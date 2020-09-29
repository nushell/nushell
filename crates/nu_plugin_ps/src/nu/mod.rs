use crate::ps::{ps, Ps};
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnSuccess, ReturnValue, Signature, Value};

use futures::executor::block_on;

impl Plugin for Ps {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("ps")
            .desc("View information about system processes.")
            .switch(
                "long",
                "list all available columns for each entry",
                Some('l'),
            )
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(ps(callinfo.name_tag, callinfo.args.has("long")))?
            .into_iter()
            .map(ReturnSuccess::value)
            .collect())
    }

    fn filter(&mut self, _: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}
