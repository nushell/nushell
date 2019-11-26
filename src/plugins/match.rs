use nu::{serve_plugin, Plugin};
use nu_errors::ShellError;
use nu_protocol::{
    CallInfo, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, UntaggedValue, Value,
};

use regex::Regex;

struct Match {
    column: String,
    regex: Regex,
}

impl Match {
    fn new() -> Self {
        Match {
            column: String::new(),
            regex: Regex::new("").unwrap(),
        }
    }
}

impl Plugin for Match {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("match")
            .desc("filter rows by regex")
            .required("member", SyntaxShape::Member, "the column name to match")
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
                    ..
                } => {
                    self.regex = Regex::new(s).unwrap();
                }
                Value { tag, .. } => {
                    return Err(ShellError::labeled_error(
                        "Unrecognized type in params",
                        "value",
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
                    match val {
                        Value {
                            value: UntaggedValue::Primitive(Primitive::String(s)),
                            ..
                        } => {
                            flag = self.regex.is_match(s);
                        }
                        Value { tag, .. } => {
                            return Err(ShellError::labeled_error("expected string", "value", tag));
                        }
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

fn main() {
    serve_plugin(&mut Match::new());
}
