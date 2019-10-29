use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tagged, TaggedDictBuilder, Value,
};

use nom::{
    bytes::complete::{tag, take_while},
    IResult,
};
use regex::Regex;

#[derive(Debug)]
enum ReadCommand {
    Text(String),
    Column(String),
}

fn read(input: &str) -> IResult<&str, Vec<ReadCommand>> {
    let mut output = vec![];

    let mut loop_input = input;
    loop {
        let (input, before) = take_while(|c| c != '{')(loop_input)?;
        if before.len() > 0 {
            output.push(ReadCommand::Text(before.to_string()));
        }
        if input != "" {
            // Look for column as we're now at one
            let (input, _) = tag("{")(input)?;
            let (input, column) = take_while(|c| c != '}')(input)?;
            let (input, _) = tag("}")(input)?;

            output.push(ReadCommand::Column(column.to_string()));
            loop_input = input;
        } else {
            loop_input = input;
        }
        if loop_input == "" {
            break;
        }
    }

    Ok((loop_input, output))
}

fn column_names(commands: &[ReadCommand]) -> Vec<String> {
    let mut output = vec![];

    for command in commands {
        match command {
            ReadCommand::Column(c) => {
                output.push(c.clone());
            }
            _ => {}
        }
    }

    output
}

fn build_regex(commands: &[ReadCommand]) -> String {
    let mut output = String::new();

    for command in commands {
        match command {
            ReadCommand::Text(s) => {
                output.push_str(&s.replace("(", "\\("));
            }
            ReadCommand::Column(_) => {
                output.push_str("(.*)");
            }
        }
    }

    return output;
}
struct Read {
    regex: Regex,
    column_names: Vec<String>,
}

impl Read {
    fn new() -> Self {
        Read {
            regex: Regex::new("").unwrap(),
            column_names: vec![],
        }
    }
}

impl Plugin for Read {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("read")
            .desc("Parse columns from string data using a simple pattern")
            .required(
                "pattern",
                SyntaxShape::Any,
                "the pattern to match. Eg) \"{foo}: {bar}\"",
            )
            .filter())
    }
    fn begin_filter(&mut self, call_info: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = call_info.args.positional {
            match &args[0] {
                Tagged {
                    item: Value::Primitive(Primitive::String(pattern)),
                    ..
                } => {
                    //self.pattern = s.clone();
                    let read_pattern = read(&pattern).unwrap();
                    let read_regex = build_regex(&read_pattern.1);

                    self.column_names = column_names(&read_pattern.1);

                    self.regex = Regex::new(&read_regex).unwrap();
                }
                Tagged { tag, .. } => {
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

    fn filter(&mut self, input: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        let mut results = vec![];
        match &input {
            Tagged {
                tag,
                item: Value::Primitive(Primitive::String(s)),
            } => {
                //self.full_input.push_str(&s);

                for cap in self.regex.captures_iter(&s) {
                    let mut dict = TaggedDictBuilder::new(tag);

                    for (idx, column_name) in self.column_names.iter().enumerate() {
                        dict.insert(column_name, Value::string(&cap[idx + 1].to_string()));
                    }

                    results.push(ReturnSuccess::value(dict.into_tagged_value()));
                }
            }
            _ => {}
        }
        Ok(results)
    }
}

fn main() {
    serve_plugin(&mut Read::new());
}
