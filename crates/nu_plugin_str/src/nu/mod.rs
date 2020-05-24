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
            .switch("capitalize", "capitalizes the string", Some('c'))
            .switch("downcase", "convert string to lowercase", Some('d'))
            .switch("upcase", "convert string to uppercase", Some('U'))
            .switch("to-int", "convert string to integer", Some('i'))
            .switch("to-float", "convert string to float", Some('F'))
            .switch("trim", "trims the string", Some('t'))
            .named(
                "replace",
                SyntaxShape::String,
                "replaces the string",
                Some('r'),
            )
            .named(
                "find-replace",
                SyntaxShape::Any,
                "finds and replaces [pattern replacement]",
                Some('f'),
            )
            .named(
                "substring",
                SyntaxShape::String,
                "convert string to portion of original, requires \"start,end\"",
                Some('s'),
            )
            .named(
                "to-date-time",
                SyntaxShape::String,
                "Convert string to Date/Time",
                Some('D'),
            )
            .rest(SyntaxShape::ColumnPath, "the column(s) to convert")
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let args = call_info.args;

        if args.has("trim") {
            self.for_trim();
        }
        if args.has("capitalize") {
            self.for_capitalize();
        }
        if args.has("downcase") {
            self.for_downcase();
        }
        if args.has("upcase") {
            self.for_upcase();
        }
        if args.has("to-int") {
            self.for_to_int();
        }
        if args.has("to-float") {
            self.for_to_float();
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

        if let Some(dt) = args.get("to-date-time") {
            let dt = dt.as_string()?;
            self.for_date_time(dt);
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
