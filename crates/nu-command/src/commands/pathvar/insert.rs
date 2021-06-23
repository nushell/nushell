// use crate::prelude::*;
// use log::trace;
// use nu_engine::WholeStreamCommand;
// use nu_errors::ShellError;
// use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
// use nu_source::Tagged;
// use nu_test_support::NATIVE_PATH_ENV_VAR;
// use std::path::{Path, PathBuf};

// pub struct SubCommand;

// impl WholeStreamCommand for SubCommand {
//     fn name(&self) -> &str {
//         "pathvar insert_at"
//     }

//     fn signature(&self) -> Signature {
//         Signature::build("pathvar insert_at").required(
//             "index",
//             SyntaxShape::Int,
//             "index at which to insert the path (starting at 0)",
//         )
//     }

//     fn usage(&self) -> &str {
//         "Add a filepath to the pathvar"
//     }

//     fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
//         insert(args)
//     }

//     fn examples(&self) -> Vec<Example> {
//         vec![
//             Example {
//                 description: "Set auto pivoting",
//                 example: "config set pivot_mode always",
//                 result: None,
//             },
//             Example {
//                 description: "Set line editor options",
//                 example: "config set line_editor [[edit_mode, completion_type]; [emacs circular]]",
//                 result: None,
//             },
//             Example {
//                 description: "Set coloring options",
//                 example: "config set color_config [[header_align header_bold]; [left $true]]",
//                 result: None,
//             },
//             Example {
//                 description: "Set nested options",
//                 example: "config set color_config.header_color white",
//                 result: None,
//             },
//         ]
//     }
// }

// pub fn insert(args: CommandArgs) -> Result<OutputStream, ShellError> {
//     let ctx = &args.context;
//     // ctx.scope.enter_scope();

//     let index_arg: Tagged<u64> = args.req(0)?;
//     let index = index_arg.item as usize;
//     let input: Vec<Value> = args.input.collect();
//     let pathvar: Vec<&str> = ctx
//         .scope
//         .get_env(NATIVE_PATH_ENV_VAR)
//         .unwrap()
//         .split(":")
//         .collect();

//     if input.len() == 0 {
//         return Err(ShellError::labeled_error("no input", "no input", None));
//     } else if input.len() == 1 {
//         pathvar.insert(index, input[0].expect_string());
//         ctx.scope
//             .add_env_var(NATIVE_PATH_ENV_VAR, pathvar.join(":"));
//     } else {
//         let iter = input.iter().map(|v| v.expect_string());
//         let new_pathvar = pathvar[0..index].into_iter().chain(iter);
//         // .chain(pathvar[index..].iter());
//         ctx.scope
//             .add_env_var(NATIVE_PATH_ENV_VAR, new_pathvar.join(":"));
//     }
//     return Ok(OutputStream::empty());
//     trace!("input: {:?}", input);

//     let index = index_arg.item;

//     // let path = path_to_add.item.into_os_string().into_string();

//     // if let Ok(mut path) = path {
//     //     path.push(':');
//     //     let old_pathvar = ctx.scope.get_env(NATIVE_PATH_ENV_VAR).unwrap();
//     //     let paths: Vec<String> = old_pathvar.split(":").collect();
//     //     assert!()

//     //     path.push_str(&old_pathvar);
//     //     ctx.scope.add_env_var(NATIVE_PATH_ENV_VAR, path);
//     //     Ok(OutputStream::empty())
//     // } else {
//     //     Err(ShellError::labeled_error(
//     //         "Invalid path.",
//     //         "cannot convert to string",
//     //         path_to_add.tag,
//     //     ))
//     // }
//     Ok(OutputStream::empty())
// }
