use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::{AnchorLocation, Span, Tagged};
use std::path::{Path, PathBuf};

pub struct Open;

#[derive(Deserialize)]
pub struct OpenArgs {
    path: Tagged<PathBuf>,
    raw: Tagged<bool>,
}

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
    }

    fn usage(&self) -> &str {
        "Load a file into a cell, convert to table if possible (avoid by appending '--raw')"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, open)?.run()
    }
}

fn open(
    OpenArgs { path, raw }: OpenArgs,
    RunnableContext { shell_manager, .. }: RunnableContext,
) -> Result<OutputStream, ShellError> {
    let cwd = PathBuf::from(shell_manager.path());
    let full_path = cwd;

    let stream = async_stream! {

        let result = fetch(&full_path, &path.item, path.tag.span).await;

        if let Err(e) = result {
            yield Err(e);
            return;
        }
        let (file_extension, contents, contents_tag) = result?;

        let file_extension = if raw.item {
            None
        } else {
            // If the extension could not be determined via mimetype, try to use the path
            // extension. Some file types do not declare their mimetypes (such as bson files).
            file_extension.or(path.extension().map(|x| x.to_string_lossy().to_string()))
        };

        let tagged_contents = contents.into_value(&contents_tag);

        if let Some(extension) = file_extension {
            yield Ok(ReturnSuccess::Action(CommandAction::AutoConvert(tagged_contents, extension)))
        } else {
            yield ReturnSuccess::value(tagged_contents);
        }
    };

    Ok(stream.to_output_stream())
}

pub async fn fetch(
    cwd: &PathBuf,
    location: &PathBuf,
    span: Span,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    let mut cwd = cwd.clone();

    cwd.push(Path::new(location));
    if let Ok(cwd) = dunce::canonicalize(cwd) {
        match std::fs::read(&cwd) {
            Ok(bytes) => match std::str::from_utf8(&bytes) {
                Ok(s) => Ok((
                    cwd.extension()
                        .map(|name| name.to_string_lossy().to_string()),
                    UntaggedValue::string(s),
                    Tag {
                        span,
                        anchor: Some(AnchorLocation::File(cwd.to_string_lossy().to_string())),
                    },
                )),
                Err(_) => {
                    //Non utf8 data.
                    match (bytes.get(0), bytes.get(1)) {
                        (Some(x), Some(y)) if *x == 0xff && *y == 0xfe => {
                            // Possibly UTF-16 little endian
                            let utf16 = read_le_u16(&bytes[2..]);

                            if let Some(utf16) = utf16 {
                                match std::string::String::from_utf16(&utf16) {
                                    Ok(s) => Ok((
                                        cwd.extension()
                                            .map(|name| name.to_string_lossy().to_string()),
                                        UntaggedValue::string(s),
                                        Tag {
                                            span,
                                            anchor: Some(AnchorLocation::File(
                                                cwd.to_string_lossy().to_string(),
                                            )),
                                        },
                                    )),
                                    Err(_) => Ok((
                                        None,
                                        UntaggedValue::binary(bytes),
                                        Tag {
                                            span,
                                            anchor: Some(AnchorLocation::File(
                                                cwd.to_string_lossy().to_string(),
                                            )),
                                        },
                                    )),
                                }
                            } else {
                                Ok((
                                    None,
                                    UntaggedValue::binary(bytes),
                                    Tag {
                                        span,
                                        anchor: Some(AnchorLocation::File(
                                            cwd.to_string_lossy().to_string(),
                                        )),
                                    },
                                ))
                            }
                        }
                        (Some(x), Some(y)) if *x == 0xfe && *y == 0xff => {
                            // Possibly UTF-16 big endian
                            let utf16 = read_be_u16(&bytes[2..]);

                            if let Some(utf16) = utf16 {
                                match std::string::String::from_utf16(&utf16) {
                                    Ok(s) => Ok((
                                        cwd.extension()
                                            .map(|name| name.to_string_lossy().to_string()),
                                        UntaggedValue::string(s),
                                        Tag {
                                            span,
                                            anchor: Some(AnchorLocation::File(
                                                cwd.to_string_lossy().to_string(),
                                            )),
                                        },
                                    )),
                                    Err(_) => Ok((
                                        None,
                                        UntaggedValue::binary(bytes),
                                        Tag {
                                            span,
                                            anchor: Some(AnchorLocation::File(
                                                cwd.to_string_lossy().to_string(),
                                            )),
                                        },
                                    )),
                                }
                            } else {
                                Ok((
                                    None,
                                    UntaggedValue::binary(bytes),
                                    Tag {
                                        span,
                                        anchor: Some(AnchorLocation::File(
                                            cwd.to_string_lossy().to_string(),
                                        )),
                                    },
                                ))
                            }
                        }
                        _ => Ok((
                            None,
                            UntaggedValue::binary(bytes),
                            Tag {
                                span,
                                anchor: Some(AnchorLocation::File(
                                    cwd.to_string_lossy().to_string(),
                                )),
                            },
                        )),
                    }
                }
            },
            Err(_) => Err(ShellError::labeled_error(
                "File could not be opened",
                "file not found",
                span,
            )),
        }
    } else {
        Err(ShellError::labeled_error(
            "File could not be opened",
            "file not found",
            span,
        ))
    }
}

fn read_le_u16(input: &[u8]) -> Option<Vec<u16>> {
    if input.len() % 2 != 0 || input.len() < 2 {
        None
    } else {
        let mut result = vec![];
        let mut pos = 0;
        while pos < input.len() {
            result.push(u16::from_le_bytes([input[pos], input[pos + 1]]));
            pos += 2;
        }

        Some(result)
    }
}

fn read_be_u16(input: &[u8]) -> Option<Vec<u16>> {
    if input.len() % 2 != 0 || input.len() < 2 {
        None
    } else {
        let mut result = vec![];
        let mut pos = 0;
        while pos < input.len() {
            result.push(u16::from_be_bytes([input[pos], input[pos + 1]]));
            pos += 2;
        }

        Some(result)
    }
}
