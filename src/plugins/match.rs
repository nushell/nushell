use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, Value,
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
            .required("member", SyntaxShape::Member)
            .required("regex", SyntaxShape::String)
            .filter())
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.column = s.clone();
                }
                _ => {
                    return Err(ShellError::string(format!(
                        "Unrecognized type in params: {:?}",
                        args[0]
                    )));
                }
            }
            match &args[1] {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => {
                    self.regex = Regex::new(s).unwrap();
                }
                _ => {
                    return Err(ShellError::string(format!(
                        "Unrecognized type in params: {:?}",
                        args[1]
                    )));
                }
            }
        }
        Ok(vec![])
    }

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        let flag: bool;
        match &input {
            Tagged {
                item: Value::Row(dict),
                ..
            } => {
                if let Some(val) = dict.entries.get(&self.column) {
                    match val {
                        Tagged {
                            item: Value::Primitive(Primitive::String(s)),
                            ..
                        } => {
                            flag = self.regex.is_match(s);
                        }
                        _ => {
                            return Err(ShellError::string(format!(
                                "value is not a string! {:?}",
                                &val
                            )));
                        }
                    }
                } else {
                    return Err(ShellError::string(format!(
                        "column not in row! {:?} {:?}",
                        &self.column, dict
                    )));
                }
            }
            _ => {
                return Err(ShellError::string(format!("Not a row! {:?}", &input)));
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
