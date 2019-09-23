use crate::commands::WholeStreamCommand;
use crate::data::meta::Span;
use crate::data::Value;
use crate::errors::ShellError;
use crate::parser::registry::Signature;
use crate::prelude::*;
use indexmap::IndexMap;
use std::process::Command;
use std::str;

pub struct Docker;

#[derive(Deserialize)]
pub struct DockerArgs {
    sub_command: Tagged<String>,
    rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Docker {
    fn name(&self) -> &str {
        "docker"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("sub_command", SyntaxShape::Member)
            .rest(SyntaxShape::Member)
    }

    fn usage(&self) -> &str {
        "e.g. docker ps, docker images"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, docker_arg)?.run()
        // docker(args, registry)
    }
}
pub fn docker_arg(
    DockerArgs {
        sub_command,
        rest: _fields,
    }: DockerArgs,
    RunnableContext { input: _, name, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    match sub_command.item().as_str() {
        "ps" => docker_ps(name),
        "images" => docker_images(name),
        _ => Err(ShellError::labeled_error(
            "Unsupported Docker command",
            format!("'{}'?", sub_command.item()),
            Span::unknown(),
        )),
    }
}

fn process_docker_output(cmd_output: &str, tag: Tag) -> Result<OutputStream, ShellError> {
    let mut docker_out = VecDeque::new();
    let columns: Vec<&str> = cmd_output.lines().collect();

    let header: Vec<&str> = columns
        .iter()
        .take(1)
        .next()
        .unwrap()
        .split_whitespace()
        .collect();

    for line in columns.iter().skip(1) {
        let values: Vec<&str> = line
            .trim_end()
            .split("  ") // Some columns values contains spaces to split by two spaces
            .filter(|s| s.trim() != "")
            .collect();

        let mut indexmap = IndexMap::new();
        for (i, v) in values.iter().enumerate() {
            indexmap.insert(
                header[i].to_string(),
                Value::string(v.trim().to_string()).tagged(tag),
            );
        }

        docker_out.push_back(Value::Row(indexmap.into()).tagged(tag))
    }

    Ok(docker_out.to_output_stream())
}

pub fn docker_images(tag: Tag) -> Result<OutputStream, ShellError> {
    let output = Command::new("docker")
        .arg("images")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output, tag);

    out
}

pub fn docker_ps(tag: Tag) -> Result<OutputStream, ShellError> {
    let output = Command::new("docker")
        .arg("ps")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output, tag);

    out
}
