use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, Value,
};

use regex::Regex;

struct Parse {
    column: Option<String>,
    regex: Option<Regex>
}

impl Parse {
    fn new() -> Parse {
        Parse {
            column: None,
            regex: None
        }
    }
}

impl Plugin for Parse {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("parse")
           .desc("Create a table based off of the regex capture groups on each row.")
           .required("regex", SyntaxShape::String)
           .optional("member", SyntaxShape::Member)
           .filter())
    }

    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        let args = call_info.args;
        match args.nth(0) {
            Some(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => {
                self.regex = Regex::new(s).ok();
            }
            None => {
                return Err(ShellError::string("Invalid arguments."));
            }
            _ => {
                return Err(ShellError::string(format!("{:?} is not a regex.", args.nth(0).unwrap())));
            }
        }
        match args.nth(1) {
            Some(Tagged {
                item: Value::Primitive(Primitive::String(s)),
                ..
            }) => {
                self.column = Some(s.clone());
            }
            None => {
                self.column = None;
            }
            _ => {
                return Err(ShellError::string(format!("{:?} is not a valid column name.", args.nth(0).unwrap())));
            }
        }
        Ok(vec![])
    }
    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        match input {
            _ => {
                Ok(vec![])
            }
        }
    }
}

fn main() {
    serve_plugin(&mut Parse::new());
}
