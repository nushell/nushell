#[cfg(test)]
mod tests;

use crate::inc::{Action, SemVerAction};
use crate::Inc;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, ShellTypeName, Signature, SyntaxShape,
    UntaggedValue, Value,
};
use nu_source::{HasSpan, SpannedItem};
use nu_value_ext::ValueExt;

impl Plugin for Inc {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("inc")
            .desc("Increment a value or version. Optionally use the column of a table.")
            .switch("major", "increment the major version (eg 1.2.1 -> 2.0.0)")
            .switch("minor", "increment the minor version (eg 1.2.1 -> 1.3.0)")
            .switch("patch", "increment the patch version (eg 1.2.1 -> 1.2.2)")
            .rest(SyntaxShape::ColumnPath, "the column(s) to update")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if call_info.args.has("major") {
            self.for_semver(SemVerAction::Major);
        }
        if call_info.args.has("minor") {
            self.for_semver(SemVerAction::Minor);
        }
        if call_info.args.has("patch") {
            self.for_semver(SemVerAction::Patch);
        }

        if let Some(args) = call_info.args.positional {
            for arg in args {
                match arg {
                    table @ Value {
                        value: UntaggedValue::Primitive(Primitive::ColumnPath(_)),
                        ..
                    } => {
                        self.field = Some(table.as_column_path()?);
                    }
                    value => {
                        return Err(ShellError::type_error(
                            "table",
                            value.type_name().spanned(value.span()),
                        ))
                    }
                }
            }
        }

        if self.action.is_none() {
            self.action = Some(Action::Default);
        }

        match &self.error {
            Some(reason) => Err(ShellError::untagged_runtime_error(format!(
                "{}: {}",
                reason,
                Inc::usage()
            ))),
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.inc(input)?)])
    }
}
