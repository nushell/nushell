use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, ReturnSuccess, ReturnValue, Signature, Value};

use crate::Sum;

impl Plugin for Sum {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("sum")
            .desc("Sum a column of values.")
            .filter())
    }

    fn begin_filter(&mut self, _: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.sum(input)?;
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        match self.total {
            None => Ok(vec![]),
            Some(ref v) => Ok(vec![ReturnSuccess::value(v.clone())]),
        }
    }
}
