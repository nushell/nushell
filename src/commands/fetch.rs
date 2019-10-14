use crate::commands::UnevaluatedCallInfo;
use crate::context::AnchorLocation;
use crate::data::meta::Span;
use crate::data::Value;
use crate::errors::ShellError;
use crate::parser::hir::SyntaxShape;
use crate::parser::registry::Signature;
use crate::prelude::*;
use mime::Mime;
use std::path::PathBuf;
use std::str::FromStr;
use surf::mime;
use uuid::Uuid;
pub struct Fetch;

impl PerItemCommand for Fetch {
    fn name(&self) -> &str {
        "fetch"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("path", SyntaxShape::Path)
            .switch("raw")
    }

    fn usage(&self) -> &str {
        "Load from a URL into a cell, convert to table if possible (avoid by appending '--raw')"
    }

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        _input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError> {
        run(call_info, registry, raw_args)
    }
}

fn run(
    call_info: &CallInfo,
    registry: &CommandRegistry,
    raw_args: &RawCommandArgs,
) -> Result<OutputStream, ShellError> {
    let path = match call_info
        .args
        .nth(0)
        .ok_or_else(|| ShellError::string(&format!("No file or directory specified")))?
    {
        file => file,
    };
    let path_buf = path.as_path()?;
    let path_str = path_buf.display().to_string();
    let path_span = path.tag.span;
    let has_raw = call_info.args.has("raw");
    let registry = registry.clone();
    let raw_args = raw_args.clone();

    let stream = async_stream! {

        let result = fetch(&path_str, path_span).await;

        if let Err(e) = result {
            yield Err(e);
            return;
        }
        let (file_extension, contents, contents_tag, anchor_location) = result.unwrap();

        let file_extension = if has_raw {
            None
        } else {
            // If the extension could not be determined via mimetype, try to use the path
            // extension. Some file types do not declare their mimetypes (such as bson files).
            file_extension.or(path_str.split('.').last().map(String::from))
        };

        if contents_tag.anchor != uuid::Uuid::nil() {
            // If we have loaded something, track its source
            yield ReturnSuccess::action(CommandAction::AddAnchorLocation(
                contents_tag.anchor,
                anchor_location,
            ));
        }

        let tagged_contents = contents.tagged(contents_tag);

        if let Some(extension) = file_extension {
            let command_name = format!("from-{}", extension);
            if let Some(converter) = registry.get_command(&command_name) {
                let new_args = RawCommandArgs {
                    host: raw_args.host,
                    shell_manager: raw_args.shell_manager,
                    call_info: UnevaluatedCallInfo {
                        args: crate::parser::hir::Call {
                            head: raw_args.call_info.args.head,
                            positional: None,
                            named: None
                        },
                        source: raw_args.call_info.source,
                        source_map: raw_args.call_info.source_map,
                        name_tag: raw_args.call_info.name_tag,
                    }
                };
                let mut result = converter.run(new_args.with_input(vec![tagged_contents]), &registry, false);
                let result_vec: Vec<Result<ReturnSuccess, ShellError>> = result.drain_vec().await;
                for res in result_vec {
                    match res {
                        Ok(ReturnSuccess::Value(Tagged { item: Value::Table(list), ..})) => {
                            for l in list {
                                yield Ok(ReturnSuccess::Value(l));
                            }
                        }
                        Ok(ReturnSuccess::Value(Tagged { item, .. })) => {
                            yield Ok(ReturnSuccess::Value(Tagged { item, tag: contents_tag }));
                        }
                        x => yield x,
                    }
                }
            } else {
                yield ReturnSuccess::value(tagged_contents);
            }
        } else {
            yield ReturnSuccess::value(tagged_contents);
        }
    };

    Ok(stream.to_output_stream())
}

pub async fn fetch(
    location: &str,
    span: Span,
) -> Result<(Option<String>, Value, Tag, AnchorLocation), ShellError> {
    if let Err(_) = url::Url::parse(location) {
        return Err(ShellError::labeled_error(
            "Incomplete or incorrect url",
            "expected a full url",
            span,
        ));
    }

    let response = surf::get(location).await;
    match response {
        Ok(mut r) => match r.headers().get("content-type") {
            Some(content_type) => {
                let content_type = Mime::from_str(content_type).unwrap();
                match (content_type.type_(), content_type.subtype()) {
                    (mime::APPLICATION, mime::XML) => Ok((
                        Some("xml".to_string()),
                        Value::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Uuid::new_v4(),
                        },
                        AnchorLocation::Url(location.to_string()),
                    )),
                    (mime::APPLICATION, mime::JSON) => Ok((
                        Some("json".to_string()),
                        Value::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Uuid::new_v4(),
                        },
                        AnchorLocation::Url(location.to_string()),
                    )),
                    (mime::APPLICATION, mime::OCTET_STREAM) => {
                        let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load binary file",
                                "could not load",
                                span,
                            )
                        })?;
                        Ok((
                            None,
                            Value::binary(buf),
                            Tag {
                                span,
                                anchor: Uuid::new_v4(),
                            },
                            AnchorLocation::Url(location.to_string()),
                        ))
                    }
                    (mime::IMAGE, mime::SVG) => Ok((
                        Some("svg".to_string()),
                        Value::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load svg from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Uuid::new_v4(),
                        },
                        AnchorLocation::Url(location.to_string()),
                    )),
                    (mime::IMAGE, image_ty) => {
                        let buf: Vec<u8> = r.body_bytes().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load image file",
                                "could not load",
                                span,
                            )
                        })?;
                        Ok((
                            Some(image_ty.to_string()),
                            Value::binary(buf),
                            Tag {
                                span,
                                anchor: Uuid::new_v4(),
                            },
                            AnchorLocation::Url(location.to_string()),
                        ))
                    }
                    (mime::TEXT, mime::HTML) => Ok((
                        Some("html".to_string()),
                        Value::string(r.body_string().await.map_err(|_| {
                            ShellError::labeled_error(
                                "Could not load text from remote url",
                                "could not load",
                                span,
                            )
                        })?),
                        Tag {
                            span,
                            anchor: Uuid::new_v4(),
                        },
                        AnchorLocation::Url(location.to_string()),
                    )),
                    (mime::TEXT, mime::PLAIN) => {
                        let path_extension = url::Url::parse(location)
                            .unwrap()
                            .path_segments()
                            .and_then(|segments| segments.last())
                            .and_then(|name| if name.is_empty() { None } else { Some(name) })
                            .and_then(|name| {
                                PathBuf::from(name)
                                    .extension()
                                    .map(|name| name.to_string_lossy().to_string())
                            });

                        Ok((
                            path_extension,
                            Value::string(r.body_string().await.map_err(|_| {
                                ShellError::labeled_error(
                                    "Could not load text from remote url",
                                    "could not load",
                                    span,
                                )
                            })?),
                            Tag {
                                span,
                                anchor: Uuid::new_v4(),
                            },
                            AnchorLocation::Url(location.to_string()),
                        ))
                    }
                    (ty, sub_ty) => Ok((
                        None,
                        Value::string(format!("Not yet supported MIME type: {} {}", ty, sub_ty)),
                        Tag {
                            span,
                            anchor: Uuid::new_v4(),
                        },
                        AnchorLocation::Url(location.to_string()),
                    )),
                }
            }
            None => Ok((
                None,
                Value::string(format!("No content type found")),
                Tag {
                    span,
                    anchor: Uuid::new_v4(),
                },
                AnchorLocation::Url(location.to_string()),
            )),
        },
        Err(_) => {
            return Err(ShellError::labeled_error(
                "URL could not be opened",
                "url not found",
                span,
            ));
        }
    }
}
