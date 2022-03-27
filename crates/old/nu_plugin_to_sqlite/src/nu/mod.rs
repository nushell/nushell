#[cfg(test)]
mod tests;

use crate::ToSqlite;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{ReturnValue, Signature, Value};
use nu_source::Tag;

impl Plugin for ToSqlite {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("to sqlite")
            .usage("Convert table into sqlite binary")
            .filter())
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        self.state.push(input);
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        crate::to_sqlite::to_sqlite(self.state.clone(), Tag::unknown())
    }
}
