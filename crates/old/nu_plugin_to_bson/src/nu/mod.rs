#[cfg(test)]
mod tests;

use crate::ToBson;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{ReturnValue, Signature, Value};
use nu_source::Tag;

impl Plugin for ToBson {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("to bson")
            .usage("Convert table into .bson binary")
            .filter())
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.state.push(input);
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(crate::to_bson::to_bson(self.state.clone(), Tag::unknown()))
    }
}
