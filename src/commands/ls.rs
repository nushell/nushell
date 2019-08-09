use crate::errors::ShellError;
use crate::prelude::*;

//pub fn ls(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
// let args = args.evaluate_once(registry)?;
// let path = PathBuf::from(args.shell_manager.path());
// let mut full_path = PathBuf::from(path);
// match &args.nth(0) {
//     Some(Tagged {
//         item: Value::Primitive(Primitive::String(s)),
//         ..
//     }) => full_path.push(Path::new(&s)),
//     _ => {}
// }

// let entries = std::fs::read_dir(&full_path);

// let entries = match entries {
//     Err(e) => {
//         if let Some(s) = args.nth(0) {
//             return Err(ShellError::labeled_error(
//                 e.to_string(),
//                 e.to_string(),
//                 s.span(),
//             ));
//         } else {
//             return Err(ShellError::labeled_error(
//                 e.to_string(),
//                 e.to_string(),
//                 args.name_span(),
//             ));
//         }
//     }
//     Ok(o) => o,
// };

// let mut shell_entries = VecDeque::new();

// for entry in entries {
//     let entry = entry?;
//     let filepath = entry.path();
//     let filename = filepath.strip_prefix(&full_path).unwrap();
//     let value = dir_entry_dict(
//         filename,
//         &entry.metadata()?,
//         Tag::unknown_origin(args.call_info.name_span),
//     )?;
//     shell_entries.push_back(ReturnSuccess::value(value))
// }
// Ok(shell_entries.to_output_stream())

pub fn ls(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let shell_manager = args.shell_manager.clone();
    let args = args.evaluate_once(registry)?;
    shell_manager.ls(args)
}
