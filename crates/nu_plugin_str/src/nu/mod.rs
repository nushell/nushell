#[cfg(test)]
mod tests;

use crate::strutils::ReplaceAction;
use crate::Str;
use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, ShellTypeName, Signature, SyntaxShape,
    UntaggedValue, Value,
};
use nu_value_ext::ValueExt;

impl Plugin for Str {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("str")
            .desc("Apply string function. Optional use the column of a table")
            .switch("downcase", "convert string to lowercase")
            .switch("upcase", "convert string to uppercase")
            .switch("to-int", "convert string to integer")
            .named("replace", SyntaxShape::String, "replaces the string")
            .named(
                "find-replace",
                SyntaxShape::Any,
                "finds and replaces [pattern replacement]",
            )
            .named(
                "substring",
                SyntaxShape::String,
                "convert string to portion of original, requires \"start,end\"",
            )
            .rest(SyntaxShape::ColumnPath, "the column(s) to convert")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let args = call_info.args;

        if args.has("downcase") {
            self.for_downcase();
        }
        if args.has("upcase") {
            self.for_upcase();
        }
        if args.has("to-int") {
            self.for_to_int();
        }
        if args.has("substring") {
            if let Some(start_end) = args.get("substring") {
                match start_end {
                    Value {
                        value: UntaggedValue::Primitive(Primitive::String(s)),
                        ..
                    } => {
                        self.for_substring(s.to_string())?;
                    }
                    _ => {
                        return Err(ShellError::labeled_error(
                            "Unrecognized type in params",
                            start_end.type_name(),
                            &start_end.tag,
                        ))
                    }
                }
            }
        }
        if args.has("replace") {
            if let Some(Value {
                value: UntaggedValue::Primitive(Primitive::String(replacement)),
                ..
            }) = args.get("replace")
            {
                self.for_replace(ReplaceAction::Direct(replacement.clone()));
            }
        }

        if args.has("find-replace") {
            if let Some(Value {
                value: UntaggedValue::Table(arguments),
                tag,
            }) = args.get("find-replace")
            {
                self.for_replace(ReplaceAction::FindAndReplace(
                    arguments
                        .get(0)
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "expected file and replace strings eg) [find replace]",
                                "missing find-replace values",
                                tag,
                            )
                        })?
                        .as_string()?,
                    arguments
                        .get(1)
                        .ok_or_else(|| {
                            ShellError::labeled_error(
                                "expected file and replace strings eg) [find replace]",
                                "missing find-replace values",
                                tag,
                            )
                        })?
                        .as_string()?,
                ));
            }
        }

        if let Some(possible_field) = args.nth(0) {
            let possible_field = possible_field.as_column_path()?;
            self.for_field(possible_field);
        }

        match &self.error {
            Some(reason) => Err(ShellError::untagged_runtime_error(format!(
                "{}: {}",
                reason,
                Str::usage()
            ))),
            None => Ok(vec![]),
        }
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![ReturnSuccess::value(self.strutils(input)?)])
    }
}
