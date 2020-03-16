use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::AnchorLocation;

pub struct ToHTML;

impl WholeStreamCommand for ToHTML {
    fn name(&self) -> &str {
        "to-html"
    }

    fn signature(&self) -> Signature {
        Signature::build("to-html")
    }

    fn usage(&self) -> &str {
        "Convert table into simple HTML"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_html(args, registry)
    }
}

fn to_html(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let name_tag = args.name_tag();
    //let name_span = name_tag.span;
    let stream = async_stream! {
        let input: Vec<Value> = args.input.values.collect().await;
        let headers = nu_protocol::merge_descriptors(&input);
        let mut output_string = "<html><body>".to_string();

        if !headers.is_empty() && (headers.len() > 1 || headers[0] != "<value>") {
            output_string.push_str("<table>");

            output_string.push_str("<tr>");
            for header in &headers {
                output_string.push_str("<th>");
                output_string.push_str(&htmlescape::encode_minimal(&header));
                output_string.push_str("</th>");
            }
            output_string.push_str("</tr>");
        }

        for row in input {
            match row.value {
                UntaggedValue::Primitive(Primitive::Binary(b)) => {
                    // This might be a bit much, but it's fun :)
                    match row.tag.anchor {
                        Some(AnchorLocation::Url(f)) |
                        Some(AnchorLocation::File(f)) => {
                            let extension = f.split('.').last().map(String::from);
                            match extension {
                                Some(s) if ["png", "jpg", "bmp", "gif", "tiff", "jpeg"].contains(&s.as_str()) => {
                                    output_string.push_str("<img src=\"data:image/");
                                    output_string.push_str(&s);
                                    output_string.push_str(";base64,");
                                    output_string.push_str(&base64::encode(&b));
                                    output_string.push_str("\">");
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                UntaggedValue::Primitive(Primitive::String(ref b)) => {
                    // This might be a bit much, but it's fun :)
                    match row.tag.anchor {
                        Some(AnchorLocation::Url(f)) |
                        Some(AnchorLocation::File(f)) => {
                            let extension = f.split('.').last().map(String::from);
                            match extension {
                                Some(s) if s == "svg" => {
                                    output_string.push_str("<img src=\"data:image/svg+xml;base64,");
                                    output_string.push_str(&base64::encode(&b.as_bytes()));
                                    output_string.push_str("\">");
                                    continue;
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                    output_string.push_str(&(htmlescape::encode_minimal(&format_leaf(&row.value).plain_string(100_000)).replace("\n", "<br>")));
                }
                UntaggedValue::Row(row) => {
                    output_string.push_str("<tr>");
                    for header in &headers {
                        let data = row.get_data(header);
                        output_string.push_str("<td>");
                        output_string.push_str(&format_leaf(data.borrow()).plain_string(100_000));
                        output_string.push_str("</td>");
                    }
                    output_string.push_str("</tr>");
                }
                p => {
                    output_string.push_str(&(htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000)).replace("\n", "<br>")));
                }
            }
        }

        if !headers.is_empty() && (headers.len() > 1 || headers[0] != "<value>") {
            output_string.push_str("</table>");
        }
        output_string.push_str("</body></html>");

        yield ReturnSuccess::value(UntaggedValue::string(output_string).into_value(name_tag));
    };

    Ok(stream.to_output_stream())
}
