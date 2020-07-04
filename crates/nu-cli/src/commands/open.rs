use crate::commands::classified::maybe_text_codec::{MaybeTextCodec, StringOrBinary};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use futures_codec::FramedRead;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{AnchorLocation, Span, Tagged};
use std::path::PathBuf;
extern crate encoding_rs;
use crate::commands::constants::BAT_LANGUAGES;
use encoding_rs::*;
use futures::prelude::*;
use log::debug;
use std::fs::File;

pub struct Open;

#[derive(Deserialize)]
pub struct OpenArgs {
    path: Tagged<PathBuf>,
    raw: Tagged<bool>,
    encoding: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for Open {
    fn name(&self) -> &str {
        "open"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "path",
                SyntaxShape::Path,
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
        r#"Load a file into a cell, convert to table if possible (avoid by appending '--raw').
        
Multiple encodings are supported for reading text files by using
the '--encoding <encoding>' parameter. Here is an example of a few:
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5

For a more complete list of encodings please refer to the encoding_rs
documentation link at https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics"#
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        open(args, registry).await
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

async fn open(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let cwd = PathBuf::from(args.shell_manager.path());
    let registry = registry.clone();

    let (
        OpenArgs {
            path,
            raw,
            encoding,
        },
        _,
    ) = args.process(&registry).await?;

    // TODO: Remove once Streams are supported everywhere!
    // As a short term workaround for getting AutoConvert and Bat functionality (Those don't currently support Streams)

    // Check if the extension has a "from *" command OR "bat" supports syntax highlighting
    // AND the user doesn't want the raw output
    // In these cases, we will collect the Stream
    let ext = if raw.item {
        None
    } else {
        path.extension()
            .map(|name| name.to_string_lossy().to_string())
    };

    if let Some(ext) = ext {
        // Check if we have a conversion command
        if let Some(_command) = registry.get_command(&format!("from {}", ext)) {
            let (_, tagged_contents) = crate::commands::open::fetch(
                &cwd,
                &PathBuf::from(&path.item),
                path.tag.span,
                encoding,
            )
            .await?;
            return Ok(OutputStream::one(ReturnSuccess::action(
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
            )
            .await?;
            return Ok(OutputStream::one(ReturnSuccess::value(tagged_contents)));
        }
    }

    // Normal Streaming operation
    let with_encoding = if encoding.is_none() {
        None
    } else {
        Some(get_encoding(encoding)?)
    };
    let f = File::open(&path).map_err(|e| {
        ShellError::labeled_error(
            format!("Error opening file: {:?}", e),
            "Error opening file",
            path.span(),
        )
    })?;
    let async_reader = futures::io::AllowStdIo::new(f);
    let sob_stream = FramedRead::new(async_reader, MaybeTextCodec::new(with_encoding))
        .map_err(|e| ShellError::unexpected(format!("AsyncRead failed in open function: {:?}", e)))
        .into_stream();

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
            Ok(StringOrBinary::Binary(b)) => ReturnSuccess::value(
                UntaggedValue::binary(b.into_iter().collect()).into_value(file_tag),
            ),
            Err(se) => Err(se),
        }
    });

    Ok(OutputStream::new(final_stream))
}

// Note that we do not output a Stream in "fetch" since it is only used by "enter" command
// Which we expect to use a concrete Value a not a Stream
pub async fn fetch(
    cwd: &PathBuf,
    location: &PathBuf,
    span: Span,
    encoding_choice: Option<Tagged<String>>,
) -> Result<(Option<String>, Value), ShellError> {
    // TODO: I don't understand the point of this? Maybe for better error reporting
    let mut cwd = cwd.clone();
    cwd.push(location);
    let nice_location = dunce::canonicalize(&cwd).map_err(|e| {
        ShellError::labeled_error(
            format!("Cannot canonicalize file {:?} because {:?}", &cwd, e),
            "Cannot canonicalize",
            span,
        )
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

    let res = std::fs::read(location)?;

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

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Open {})
    }
}
