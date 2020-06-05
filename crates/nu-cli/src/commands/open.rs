use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature, SyntaxShape, UntaggedValue};
use nu_source::{AnchorLocation, Span, Tagged};
use std::path::{Path, PathBuf};
extern crate encoding_rs;
use encoding_rs::*;
use std::fs::File;
use std::io::Read;
use std::io::Write;

pub struct Open;

#[derive(Deserialize)]
pub struct OpenArgs {
    path: Tagged<PathBuf>,
    raw: Tagged<bool>,
    encoding: Option<String>,
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
        "Load a file into a cell, convert to table if possible (avoid by appending '--raw')"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        open(args, registry)
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

pub fn get_encoding(opt: Option<String>) -> &'static Encoding {
    match opt {
        None => UTF_8,
        Some(label) => match Encoding::for_label((&label).as_bytes()) {
            None => {
                print!("{} is not a known encoding label; exiting.", label);
                std::process::exit(-2);
            }
            Some(encoding) => encoding,
        },
    }
}

fn open(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let cwd = PathBuf::from(args.shell_manager.path());
    let full_path = cwd;
    let registry = registry.clone();

    let stream = async_stream! {
        let (OpenArgs { path, raw, encoding }, _) = args.process(&registry).await?;
        let result = fetch(&full_path, &path.item, path.tag.span, encoding.unwrap()).await;

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
    encoding: String,
) -> Result<(Option<String>, UntaggedValue, Tag), ShellError> {
    let mut cwd = cwd.clone();
    let output_encoding: &Encoding = get_encoding(Some("utf-8".to_string()));
    let input_encoding: &Encoding = get_encoding(Some(encoding));
    let mut decoder = input_encoding.new_decoder();
    let mut encoder = output_encoding.new_encoder();

    let mut file: File;
    let stdout;
    let mut stdout_lock;
    let mut output = {
        stdout = std::io::stdout();
        stdout_lock = stdout.lock();
        &mut stdout_lock as &mut dyn Write
    };

    match File::open(&Path::new(&cwd)) {
        Ok(mut file) => {
            convert_via_utf8(&mut decoder, &mut encoder, &mut file, &mut output, false);
            Ok((
                cwd.extension()
                    .map(|name| name.to_string_lossy().to_string()),
                UntaggedValue::string(output.flush()),
                Tag {
                    span,
                    anchor: Some(AnchorLocation::File(cwd.to_string_lossy().to_string())),
                },
            ))
        }
        Err(_) => Err(ShellError::labeled_error(
            format!("Cannot open {:?} for reading.", &cwd),
            "file not found",
            span,
        )),
    }
    /*
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
    */
}

fn convert_via_utf8(
    decoder: &mut Decoder,
    encoder: &mut Encoder,
    read: &mut dyn Read,
    write: &mut dyn Write,
    last: bool,
) {
    let mut input_buffer = [0u8; 2048];
    let mut intermediate_buffer_bytes = [0u8; 4096];
    // Is there a safe way to create a stack-allocated &mut str?
    let mut intermediate_buffer: &mut str =
        unsafe { std::mem::transmute(&mut intermediate_buffer_bytes[..]) };
    let mut output_buffer = [0u8; 4096];
    let mut current_input_ended = false;
    while !current_input_ended {
        match read.read(&mut input_buffer) {
            Err(_) => {
                print!("Error reading input.");
                //std::process::exit(-5);
            }
            Ok(decoder_input_end) => {
                current_input_ended = decoder_input_end == 0;
                let input_ended = last && current_input_ended;
                let mut decoder_input_start = 0usize;
                loop {
                    let (decoder_result, decoder_read, decoder_written, _) = decoder.decode_to_str(
                        &input_buffer[decoder_input_start..decoder_input_end],
                        &mut intermediate_buffer,
                        input_ended,
                    );
                    decoder_input_start += decoder_read;

                    let last_output = if input_ended {
                        match decoder_result {
                            CoderResult::InputEmpty => true,
                            CoderResult::OutputFull => false,
                        }
                    } else {
                        false
                    };

                    // Regardless of whether the intermediate buffer got full
                    // or the input buffer was exhausted, let's process what's
                    // in the intermediate buffer.

                    if encoder.encoding() == UTF_8 {
                        // If the target is UTF-8, optimize out the encoder.
                        match write.write_all(&intermediate_buffer.as_bytes()[..decoder_written]) {
                            Err(_) => {
                                print!("Error writing output.");
                                //std::process::exit(-7);
                            }
                            Ok(_) => {}
                        }
                    } else {
                        let mut encoder_input_start = 0usize;
                        loop {
                            let (encoder_result, encoder_read, encoder_written, _) = encoder
                                .encode_from_utf8(
                                    &intermediate_buffer[encoder_input_start..decoder_written],
                                    &mut output_buffer,
                                    last_output,
                                );
                            encoder_input_start += encoder_read;
                            match write.write_all(&output_buffer[..encoder_written]) {
                                Err(_) => {
                                    print!("Error writing output.");
                                    //std::process::exit(-6);
                                }
                                Ok(_) => {}
                            }
                            match encoder_result {
                                CoderResult::InputEmpty => {
                                    break;
                                }
                                CoderResult::OutputFull => {
                                    continue;
                                }
                            }
                        }
                    }

                    // Now let's see if we should read again or process the
                    // rest of the current input buffer.
                    match decoder_result {
                        CoderResult::InputEmpty => {
                            break;
                        }
                        CoderResult::OutputFull => {
                            continue;
                        }
                    }
                }
            }
        }
    }
}
/*
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
*/
#[cfg(test)]
mod tests {
    use super::Open;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Open {})
    }
}
