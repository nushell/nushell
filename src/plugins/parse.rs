use nu::{
    serve_plugin, CallInfo, TaggedDictBuilder, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
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
        match input.item {
            Value::Row(dict) => {
                if let Some(col) = &self.column {
                    if let Some(Tagged {
                        item: Value::Primitive(Primitive::String(s)),
                        ..
                    }) = dict.entries.get(col.as_str()) {
                        let regex = self.regex.as_ref().ok_or_else(|| ShellError::string("no regex there"))?;
                        if regex.is_match(&s) {
                            let caps = regex.captures(&s).ok_or_else(|| ShellError::string("no captures"))?;
                            let names = regex.capture_names();
                            let mut dict = TaggedDictBuilder::new(input.tag);
                            for name in names.skip(1) {
                                let name = name.unwrap();
                                let res = caps.name(name).unwrap();
                                dict.insert(name, Value::string(res.as_str()));
                            }
                            Ok(vec![ReturnSuccess::value(dict.into_tagged_value())])
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(ShellError::string("Column not found or not filled with strings."))
                    }
                } else {
                    Err(ShellError::string("Please provide a column for now (TODO)"))
                }
            }
            _ => Ok(vec![])
        }
    }
}

fn main() {
    serve_plugin(&mut Parse::new());
}
