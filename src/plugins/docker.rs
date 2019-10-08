use futures::executor::block_on;
use nu::{
    serve_plugin, CallInfo, Plugin, Primitive, ReturnSuccess, ReturnValue, ShellError, Signature,
    SyntaxShape, Tag, Tagged, TaggedDictBuilder, Value,
};

use std::process::Command;
use std::str;

struct Docker;

impl Docker {
    fn new() -> Self {
        Self
    }
}

async fn docker(sub_command: &String, name: Tag) -> Result<Vec<Tagged<Value>>, ShellError> {
    match sub_command.as_str() {
        "ps" => docker_ps(name),
        "images" => docker_images(name),
        _ => Err(ShellError::labeled_error(
            "Unsupported Docker command",
            format!("'{}'?", sub_command),
            name.span,
        )),
    }
}

fn process_docker_output(cmd_output: &str, tag: Tag) -> Result<Vec<Tagged<Value>>, ShellError> {
    let columns: Vec<&str> = cmd_output.lines().collect();

    let header: Vec<&str> = columns
        .iter()
        .take(1)
        .next()
        .unwrap()
        .split_whitespace()
        .collect();

    let mut output = vec![];
    for line in columns.iter().skip(1) {
        let values: Vec<&str> = line
            .trim_end()
            .split("  ") // Some columns values contains spaces to split by two spaces
            .filter(|s| s.trim() != "")
            .collect();

        let mut dict = TaggedDictBuilder::new(tag);
        for (i, v) in values.iter().enumerate() {
            dict.insert(header[i].to_string(), Value::string(v.trim().to_string()));
        }

        output.push(dict.into_tagged_value());
    }

    Ok(output)
}

pub fn docker_images(tag: Tag) -> Result<Vec<Tagged<Value>>, ShellError> {
    let output = Command::new("docker")
        .arg("images")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output, tag);

    out
}

pub fn docker_ps(tag: Tag) -> Result<Vec<Tagged<Value>>, ShellError> {
    let output = Command::new("docker")
        .arg("ps")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output, tag);

    out
}

impl Plugin for Docker {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("docker")
            .required("sub_command", SyntaxShape::Member)
            .filter())
    }

    fn begin_filter(&mut self, callinfo: CallInfo) -> Result<Vec<ReturnValue>, ShellError> {
        if let Some(args) = callinfo.args.positional {
            match &args[0] {
                Tagged {
                    item: Value::Primitive(Primitive::String(s)),
                    ..
                } => match block_on(docker(&s, callinfo.name_tag)) {
                    Ok(v) => return Ok(v.into_iter().map(ReturnSuccess::value).collect()),
                    Err(e) => return Err(e),
                },
                _ => {
                    return Err(ShellError::labeled_error(format!(
                        "Unrecognized type in params: {:?}",
                        args[0]
                    )))
                }
            }
        }

        Ok(vec![])
    }

    fn filter(&mut self, _: Tagged<Value>) -> Result<Vec<ReturnValue>, ShellError> {
        Ok(vec![])
    }
}

fn main() {
    serve_plugin(&mut Docker::new());
}
