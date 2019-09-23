use crate::commands::WholeStreamCommand;
use crate::data::{Dictionary, Value};
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
            .required("sub_command", SyntaxType::Member)
            .rest(SyntaxType::Member)
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
    RunnableContext { input: _, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    // let mut docker_out = VecDeque::new();
    // docker_out.push_back(Value::Primitive(Primitive::String("docker command")));
    //
    // println!("Sub Command: {:?}", sub_command);
    // match sub_command.item() {
    //     Tagged { item: val, .. } => println!("Val: {}", val),
    //     _ => {}
    // }

    match sub_command.item().as_str() {
        "ps" => {
            // println!("ps command")
            return docker_ps();
        }
        "images" => {
            // println!("images command");
            return docker_images();
        }
        _ => {
            return Err(ShellError::labeled_error(
                "Unsupported Docker command",
                format!("'{}'?", sub_command.item()),
                Span::unknown(),
            ))
        }
    }

    // let stream = input
    //     .values
    //     .map(move |item| {
    //         let mut result = VecDeque::new();

    //         let member = vec![member.clone()];

    //         let fields = vec![&member, &fields]
    //             .into_iter()
    //             .flatten()
    //             .collect::<Vec<&Tagged<String>>>();

    //         for field in &fields {
    //             match get_member(field, &item) {
    //                 Ok(Tagged {
    //                     item: Value::Table(l),
    //                     ..
    //                 }) => {
    //                     for item in l {
    //                         result.push_back(ReturnSuccess::value(item.clone()));
    //                     }
    //                 }
    //                 Ok(x) => result.push_back(ReturnSuccess::value(x.clone())),
    //                 Err(x) => result.push_back(Err(x)),
    //             }
    //         }

    //         result
    //     })
    //     .flatten();

    // Ok(docker_out.to_output_stream())
    // Ok(docker_out.to_output_stream())
}

fn process_docker_output(cmd_output: &str) -> Result<OutputStream, ShellError> {
    let mut docker_out = VecDeque::new();
    let mut columns: Vec<&str> = cmd_output.lines().collect();
    // println!("{:#?}", columns);

    let header: Vec<&str> = columns
        .iter()
        .take(1)
        .next()
        .unwrap()
        .split_whitespace()
        .collect();

    // println!("{:#?}", header);

    columns.remove(0);

    // let span = args.call_info.name_span;
    for line in columns {
        let values: Vec<&str> = line
            .trim_end()
            .split("  ") // Some columns values contains spaces to split by two spaces
            .filter(|s| s.trim() != "")
            .collect();

        // println!("len: {}", values.len());
        // println!("Values: {:#?}", values);
        let mut indexmap = IndexMap::new();
        for (i, v) in values.iter().enumerate() {
            // println!("{}", i);
            // println!("{}", header[i]);
            indexmap.insert(
                header[i].to_string(),
                Tagged::from_simple_spanned_item(
                    Value::Primitive(Primitive::String(v.trim().to_string())),
                    Span::unknown(),
                ),
            );
        }

        docker_out.push_back(Tagged::from_simple_spanned_item(
            Value::Row(Dictionary::from(indexmap)),
            Span::unknown(),
        ))
    }

    // let t = dict.into_tagged_value();

    // docker_out.push_back(ReturnSuccess::value(t));

    Ok(docker_out.to_output_stream())
}

pub fn docker_images() -> Result<OutputStream, ShellError> {
    // let mut docker_out = VecDeque::new();
    // docker_out.push_back(Value::Primitive(Primitive::String("docker command")));
    // Ok(docker_out.to_output_stream())
    //
    // let mut dict = TaggedDictBuilder::new(Tag::unknown_origin(cmd_args.call_info.name_span));
    // dict.insert("name", Value::string("test name"));
    // println!("{:#?}", cmd_args.call_info);

    // let args = cmd_args.evaluate_once(registry)?;
    // println!("{:#?}", args.call_info);

    // let arg = args.nth(0);
    // println!("{:?}", arg);

    // match &args.nth(0) {
    //     Some(val) => println!("Val: {:?}", val),
    //     _ => {}
    // }

    let output = Command::new("docker")
        .arg("images")
        // .arg("--format")
        // .arg("table {{.ID}}\t{{.Repository}}\t{{.Tag}}\t{{.CreatedSince}}")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output);

    // let mut columns: Vec<&str> = ps_output.lines().collect();
    // // println!("{:#?}", columns);

    // let header: Vec<&str> = columns
    //     .iter()
    //     .take(1)
    //     .next()
    //     .unwrap()
    //     .split_whitespace()
    //     .collect();

    // println!("{:#?}", header);

    // columns.remove(0);

    // let span = args.call_info.name_span;
    // for line in columns {
    //     let values: Vec<&str> = line
    //         .trim_end()
    //         .split("  ") // Some columns values contains spaces to split by two spaces
    //         .filter(|s| s.trim() != "")
    //         .collect();

    //     // println!("len: {}", values.len());
    //     // println!("Values: {:#?}", values);
    //     let mut indexmap = IndexMap::new();
    //     for (i, v) in values.iter().enumerate() {
    //         // println!("{}", i);
    //         // println!("{}", header[i]);
    //         indexmap.insert(
    //             header[i].to_string(),
    //             Tagged::from_simple_spanned_item(
    //                 Value::Primitive(Primitive::String(v.trim().to_string())),
    //                 span,
    //             ),
    //         );
    //     }

    //     docker_out.push_back(Tagged::from_simple_spanned_item(
    //         Value::Row(Dictionary::from(indexmap)),
    //         span,
    //     ))
    // }

    // let t = dict.into_tagged_value();

    // docker_out.push_back(ReturnSuccess::value(t));

    // Ok(docker_out.to_output_stream())
    out
}

pub fn docker_ps() -> Result<OutputStream, ShellError> {
    let output = Command::new("docker")
        .arg("ps")
        .output()
        .expect("failed to execute process.");

    let ps_output = str::from_utf8(&output.stdout).unwrap();
    let out = process_docker_output(ps_output);

    out
}
