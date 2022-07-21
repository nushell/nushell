// use nu_protocol::{
//     ast::{Expr, Expression},
//     Span, Spanned, Type,
// };

// pub struct ExternalBuilder {
//     name: String,
//     args: Vec<String>,
// }

// impl ExternalBuilder {
//     pub fn for_name(name: &str) -> ExternalBuilder {
//         ExternalBuilder {
//             name: name.to_string(),
//             args: vec![],
//         }
//     }

//     pub fn arg(&mut self, value: &str) -> &mut Self {
//         self.args.push(value.to_string());
//         self
//     }

// pub fn build(&mut self) -> ExternalCommand {
//     let mut path = crate::fs::binaries();
//     path.push(&self.name);

//     let name = Spanned {
//         item: path.to_string_lossy().to_string(),
//         span: Span::new(0, 0),
//     };

//     let args = self
//         .args
//         .iter()
//         .map(|arg| Expression {
//             expr: Expr::String(arg.to_string()),
//             span: Span::new(0, 0),
//             ty: Type::Unknown,
//             custom_completion: None,
//         })
//         .collect::<Vec<_>>();

//     ExternalCommand {
//         name: name.to_string(),
//         name_tag: Tag::unknown(),
//         args: ExternalArgs {
//             list: args,
//             span: name.span,
//         },
//     }
// }
// }

use std::{
    io::Read,
    process::{Command, Stdio},
};

pub fn ensure_binary_present(package: &str) {
    let cargo_path = env!("CARGO");
    let mut arguments = vec!["build", "--package", package, "--quiet"];

    let profile = std::env::var("NUSHELL_CARGO_TARGET");
    if let Ok(profile) = &profile {
        arguments.push("--profile");
        arguments.push(profile);
    }

    let mut command = Command::new(cargo_path)
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo build command");

    let stderr = command.stderr.take();

    let success = command
        .wait()
        .expect("failed to wait cargo build command")
        .success();

    if let Some(mut stderr) = stderr {
        let mut buffer = String::new();
        stderr
            .read_to_string(&mut buffer)
            .expect("failed to read cargo build stderr");
        if !buffer.is_empty() {
            println!("=== cargo build stderr\n{}", buffer);
        }
    }

    if !success {
        panic!("cargo build failed");
    }
}
