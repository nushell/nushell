#[cfg(test)]
mod tests;

use crate::FromSqlite;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{CallInfo, Primitive, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tag;

// Adapted from crates/nu-command/src/commands/dataframe/utils.rs
fn convert_columns(columns: &[Value]) -> Result<Vec<String>, ShellError> {
    let res = columns
        .iter()
        .map(|value| match &value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => Ok(s.clone()),
            _ => Err(ShellError::labeled_error(
                "Incorrect column format",
                "Only string as column name",
                &value.tag,
            )),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok(res)
}

impl Plugin for FromSqlite {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("from sqlite")
            .named(
                "tables",
                SyntaxShape::Table,
                "Only convert specified tables",
                Some('t'),
            )
            .usage("Convert from sqlite binary into table")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        self.name_tag = call_info.name_tag;

        if let Some(t) = call_info.args.get("tables") {
            if let UntaggedValue::Table(columns) = t.value.clone() {
                self.tables = convert_columns(columns.as_slice())?;
            }
        }
        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        match input {
            Value {
                value: UntaggedValue::Primitive(Primitive::Binary(b)),
                ..
            } => {
                self.state.extend_from_slice(&b);
            }
            Value { tag, .. } => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Expected binary from pipeline",
                    "requires binary input",
                    self.name_tag.clone(),
                    "value originates from here",
                    tag,
                ));
            }
        }
        Ok(vec![])
    }

    fn end_filter(&mut self) -> Result<Vec<ReturnValue>, ShellError> {
        crate::from_sqlite::from_sqlite(self.state.clone(), Tag::unknown(), self.tables.clone())
    }
}
