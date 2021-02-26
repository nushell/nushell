use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};

use crate::Match;
use regex::Regex;

impl Plugin for Match {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("match")
            .desc("Filter rows by Regex pattern")
            .required("member", SyntaxShape::String, "the column name to match")
            .required("regex", SyntaxShape::String, "the regex to match with")
            .switch("insensitive", "case-insensitive search", Some('i'))
            .switch(
                "multiline",
                "multi-line mode: ^ and $ match begin/end of line",
                Some('m'),
            )
            .switch(
                "dotall",
                "dotall mode: allow a dot . to match newline character \\n",
                Some('s'),
            )
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let insensitive = call_info.args.has("insensitive");
        let multiline = call_info.args.has("multiline");
        let dotall = call_info.args.has("dotall");
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.column = s.clone();
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Unrecognized type in params",
                        "value",
                        tag,
                    ));
                }
            }
            let flags = match (insensitive, multiline, dotall) {
                (false, false, false) => "",
                (true, false, false) => "(?i)",
                (false, true, false) => "(?m)",
                (false, false, true) => "(?s)",
                (true, true, false) => "(?im)",
                (true, false, true) => "(?is)",
                (false, true, true) => "(?ms)",
                (true, true, true) => "(?ims)",
            }
            .to_owned();
            match &args[1] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    self.regex = Regex::new(&(flags + s)).map_err(|_| {
                        ShellError::labeled_error(
                            "Internal error while creating regex",
                            "internal error created by pattern",
                            tag,
                        )
                    })?;
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Unrecognized type in params",
                        "unexpected value",
                        tag,
                    ));
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, input: Value) -> Result<Vec<ReturnValue>, ShellError> {
        let flag: bool;
        match &input {
            Value {
                value: UntaggedValue::Row(dict),
                tag,
            } => {
                if let Some(val) = dict.entries.get(&self.column) {
                    if let Ok(s) = val.as_string() {
                        flag = self.regex.is_match(&s);
                    } else {
                        return Err(ShellError::labeled_error(
                            "expected string",
                            "value",
                            val.tag(),
                        ));
                    }
                } else {
                    return Err(ShellError::labeled_error(
                        format!("column not in row! {:?} {:?}", &self.column, dict),
                        "row",
                        tag,
                    ));
                }
            }
            Value { tag, .. } => {
                return Err(ShellError::labeled_error("Expected row", "value", tag));
            }
        }
        if flag {
            Ok(vec![Ok(ReturnSuccess::Value(input))])
        } else {
            Ok(vec![])
        }
    }
}
