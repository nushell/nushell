use crate::commands::viewers::BAT_LANGUAGES;
use crate::prelude::*;
use encoding_rs::{Encoding, UTF_8};

use log::debug;
use nu_engine::StringOrBinary;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_path::canonicalize;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{AnchorLocation, Span, Tagged};
use std::path::{Path, PathBuf};

pub struct Open;

impl WholeStreamCommand for Open {
    fn name(&self) -> &str {
        "open"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "path",
                SyntaxShape::FilePath,
                "the file path to load values from",
            )
            .switch(
                "raw",
                "load content as a string instead of a table",
                Some('r'),
            )
            .named(
                "encoding",
                SyntaxShape::String,
                "encoding to use to open file",
                Some('e'),
            )
    }

    fn usage(&self) -> &str {
        "Load a file into a cell, convert to table if possible (avoid by appending '--raw')."
    }

    fn extra_usage(&self) -> &str {
        r#"Multiple encodings are supported for reading text files by using
the '--encoding <encoding>' parameter. Here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.28/encoding_rs/#statics"#
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        open(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Opens \"users.csv\" and creates a table from the data",
                example: "open users.csv",
                result: None,
            },
            Example {
                description: "Opens file with iso-8859-1 encoding",
                example: "open file.csv --encoding iso-8859-1 | from csv",
                result: None,
            },
            Example {
                description: "Lists the contents of a directory (identical to `ls ../projectB`)",
                example: "open ../projectB",
                result: None,
            },
        ]
    }
}

pub fn get_encoding(opt: Option<Tagged<String>>) -> Result<&'static Encoding, ShellError> {
    match opt {
        None => Ok(UTF_8),
        Some(label) => match Encoding::for_label((&label.item).as_bytes()) {
            None => Err(ShellError::labeled_error(
                format!(
                    r#"{} is not a valid encoding, refer to https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics for a valid list of encodings"#,
                    label.item
                ),
                "invalid encoding",
                label.span(),
            )),
            Some(encoding) => Ok(encoding),
        },
    }
}

fn open(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let scope = args.scope().clone();
    let shell_manager = args.shell_manager();
    let cwd = PathBuf::from(shell_manager.path());
    let name = args.call_info.name_tag.clone();
    let ctrl_c = args.ctrl_c();

    let path: Tagged<PathBuf> = args.req(0)?;
    let raw = args.has_flag("raw");
    let encoding: Option<Tagged<String>> = args.get_flag("encoding")?;

    if path.is_dir() {
        let args = nu_engine::shell::LsArgs {
            path: Some(path),
            all: false,
            long: false,
            short_names: false,
            du: false,
        };
        return shell_manager.ls(args, name, ctrl_c);
    }

    // TODO: Remove once Streams are supported everywhere!
    // As a short term workaround for getting AutoConvert and Bat functionality (Those don't currently support Streams)

    // Check if the extension has a "from *" command OR "bat" supports syntax highlighting
    // AND the user doesn't want the raw output
    // In these cases, we will collect the Stream
    let ext = if raw {
        None
    } else {
        path.extension()
            .map(|name| name.to_string_lossy().to_string())
    };

    if let Some(ext) = ext {
        // Check if we have a conversion command
        if let Some(_command) = scope.get_command(&format!("from {}", ext)) {
            let (_, tagged_contents) = crate::commands::open::fetch(
                &cwd,
                &PathBuf::from(&path.item),
                path.tag.span,
                encoding,
            )?;
            return Ok(ActionStream::one(ReturnSuccess::action(
                CommandAction::AutoConvert(tagged_contents, ext),
            )));
        }
        // Check if bat does syntax highlighting
        if BAT_LANGUAGES.contains(&ext.as_ref()) {
            let (_, tagged_contents) = crate::commands::open::fetch(
                &cwd,
                &PathBuf::from(&path.item),
                path.tag.span,
                encoding,
            )?;
            return Ok(ActionStream::one(ReturnSuccess::value(tagged_contents)));
        }
    }

    // Normal Streaming operation
    let with_encoding = if encoding.is_none() {
        None
    } else {
        Some(get_encoding(encoding)?)
    };

    let sob_stream = shell_manager.open(&path.item, path.tag.span, with_encoding)?;

    let final_stream = sob_stream.map(move |x| {
        // The tag that will used when returning a Value
        let file_tag = Tag {
            span: path.tag.span,
            anchor: Some(AnchorLocation::File(path.to_string_lossy().to_string())),
        };

        match x {
            Ok(StringOrBinary::String(s)) => {
                ReturnSuccess::value(UntaggedValue::string(s).into_value(file_tag))
            }
            Ok(StringOrBinary::Binary(b)) => {
                ReturnSuccess::value(UntaggedValue::binary(b).into_value(file_tag))
            }
            Err(se) => Err(se),
        }
    });

    Ok(ActionStream::new(final_stream))
}

// Note that we do not output a Stream in "fetch" since it is only used by "enter" command
// Which we expect to use a concrete Value a not a Stream
pub fn fetch(
    cwd: &Path,
    location: &Path,
    span: Span,
    encoding_choice: Option<Tagged<String>>,
) -> Result<(Option<String>, Value), ShellError> {
    // TODO: I don't understand the point of this? Maybe for better error reporting
    let mut cwd = PathBuf::from(cwd);
    cwd.push(location);
    let nice_location = canonicalize(&cwd).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ShellError::labeled_error(
            format!("Cannot find file {:?}", cwd),
            "cannot find file",
            span,
        ),
        std::io::ErrorKind::PermissionDenied => {
            ShellError::labeled_error("Permission denied", "permission denied", span)
        }
        _ => ShellError::labeled_error(
            format!("Cannot open file {:?} because {:?}", &cwd, e),
            "Cannot open",
            span,
        ),
    })?;

    // The extension may be used in AutoConvert later on
    let ext = location
        .extension()
        .map(|name| name.to_string_lossy().to_string());

    // The tag that will used when returning a Value
    let file_tag = Tag {
        span,
        anchor: Some(AnchorLocation::File(
            nice_location.to_string_lossy().to_string(),
        )),
    };

    let res = std::fs::read(location)
        .map_err(|_| ShellError::labeled_error("Can't open filename given", "can't open", span))?;

    // If no encoding is provided we try to guess the encoding to read the file with
    let encoding = if encoding_choice.is_none() {
        UTF_8
    } else {
        get_encoding(encoding_choice.clone())?
    };

    // If the user specified an encoding, then do not do BOM sniffing
    let decoded_res = if encoding_choice.is_some() {
        let (cow_res, _replacements) = encoding.decode_with_bom_removal(&res);
        cow_res
    } else {
        // Otherwise, use the default UTF-8 encoder with BOM sniffing
        let (cow_res, actual_encoding, replacements) = encoding.decode(&res);
        // If we had to use replacement characters then fallback to binary
        if replacements {
            return Ok((ext, UntaggedValue::binary(res).into_value(file_tag)));
        }
        debug!("Decoded using {:?}", actual_encoding);
        cow_res
    };
    let v = UntaggedValue::string(decoded_res.to_string()).into_value(file_tag);
    Ok((ext, v))
}

#[cfg(test)]
mod tests {
    use super::Open;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Open {})
    }
}
