use crate::sys::{sysinfo, Sys};
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnSuccess, ReturnValue, Signature, Value};

use futures::executor::block_on;

impl Plugin for Sys {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("sys")
            .desc("View information about the current system.")
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(block_on(sysinfo(callinfo.name_tag))
            .into_iter()
            .map(ReturnSuccess::value)
            .collect())
    }

    fn filter(&mut self, _: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}
