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
            .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
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
            match &args[1] {
                Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag,
                } => {
                    self.regex = Regex::new(s).map_err(|_| {
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
