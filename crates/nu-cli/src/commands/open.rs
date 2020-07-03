use crate::commands::classified::maybe_text_codec::{
    guess_encoding, EncodingGuess, MaybeTextCodec, StringOrBinary,
};
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use futures_codec::FramedRead;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{AnchorLocation, Span, Tagged};
use std::path::{Path, PathBuf};
extern crate encoding_rs;
use encoding_rs::*;
use futures::prelude::*;
use log::debug;
use std::fs::File;

pub struct Open;

#[allow(dead_code)] // TODO: Still working on encoding for MaybeTextCodec
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
    let _cwd = PathBuf::from(args.shell_manager.path());
    let registry = registry.clone();

    let (
        OpenArgs {
            path,
            raw,
            encoding,
        },
        _,
    ) = args.process(&registry).await?;

    // As a short term workaround for getting AutoConvert functionality
    // Assuming the user doesn't want the raw output...

    // We will check if the extension has a "from *" command
    // If so, then we will collect the Stream so we can AutoConvert into a Value
    // Otherwise we Stream as normal
    let ext = path
        .extension()
        .map(|name| name.to_string_lossy().to_string());

    if let (Some(ext), false) = (ext, raw.item) {
        if let Some(_command) = registry.get_command(&format!("from {}", ext)) {
            let (_, tagged_contents) = crate::commands::open::fetch(
                &_cwd,
                &PathBuf::from(&path.item),
                path.tag.span,
                encoding,
            )
            .await?;
            return Ok(OutputStream::one(ReturnSuccess::action(
                CommandAction::AutoConvert(tagged_contents, ext),
            )));
        }
    }

    let with_encoding;
    if encoding.is_none() {
        with_encoding = None;
    } else {
        with_encoding = Some(get_encoding(encoding)?);
    }
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

    let final_stream = sob_stream.map(|x| match x {
        Ok(StringOrBinary::String(s)) => {
            ReturnSuccess::value(UntaggedValue::string(s).into_untagged_value())
        }
        Ok(StringOrBinary::Binary(b)) => ReturnSuccess::value(
            UntaggedValue::binary(b.into_iter().collect()).into_untagged_value(),
        ),
        Err(se) => Err(se),
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
    let mut cwd = cwd.clone();
    cwd.push(Path::new(location)); // This is so we have the correct path for reading/error reporting

    let path = dunce::canonicalize(&cwd).map_err(|e| {
        ShellError::labeled_error(
            format!("Cannot canonicalize file {:?} because {:?}", &cwd, e),
            "Cannot canonicalize",
            span,
        )
    })?;
    // The extension will be used in auto-convert later on
    let ext = path
        .extension()
        .map(|name| name.to_string_lossy().to_string());

    // The tag that will used when returning a Value
    let file_tag = Tag {
        span,
        anchor: Some(AnchorLocation::File(
            path.clone().to_string_lossy().to_string(),
        )),
    };

    let res = std::fs::read(path).map_err(|e| ShellError::from(e))?;

    // If no encoding is provided we try to guess the encoding to read the file with
    let guess: EncodingGuess;
    let encoding: &'static Encoding;
    if encoding_choice.is_none() {
        guess = guess_encoding(&res);
        encoding = UTF_8;
    } else {
        guess = EncodingGuess::Known;
        encoding = get_encoding(encoding_choice.clone())?;
    }

    // If it's a binary file we can just spit out the the results
    if let EncodingGuess::Binary = guess {
        return Ok((ext, UntaggedValue::binary(res).into_value(file_tag)));
    }

    let decoded_res;
    // If the user specified an encoding, then do not do BOM sniffing
    if encoding_choice.is_some() {
        let r = encoding.decode_with_bom_removal(&res);
        decoded_res = r.0;
    } else {
        // Otherwise, use the default UTF-8 encoder with BOM sniffing
        let r = encoding.decode(&res);
        debug!("Decoded using {:?}", r.1);
        decoded_res = r.0;
    }

    return Ok((
        ext,
        UntaggedValue::string(decoded_res.to_string()).into_value(file_tag),
    ));
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
