use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::AnchorLocation;
use std::collections::HashMap;
use regex::Regex;

pub struct ToHTML;

#[derive(Deserialize)]
pub struct ToHTMLArgs {
    with_html_color: bool,
    with_no_color: bool,
}

#[async_trait]
impl WholeStreamCommand for ToHTML {
    fn name(&self) -> &str {
        "to html"
    }

    fn signature(&self) -> Signature {
        Signature::build("to html")
            .switch(
                "html_color",
                "change ansi colors to html colors",
                Some('h'),
            )
            .switch(
                "no_color",
                "remove all ansi colors in output",
                Some('n'),
            )
    }

    fn usage(&self) -> &str {
        "Convert table into simple HTML"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_html(args, registry).await
    }
}

async fn to_html(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name_tag = args.call_info.name_tag.clone();
    let (ToHTMLArgs { with_html_color, with_no_color}, input ) = args.process(&registry).await?;
    let input: Vec<Value> = input.collect().await;
    let mut headers = nu_protocol::merge_descriptors(&input);
    let mut output_string = "<html><body>".to_string();

    // let mut hm = HashMap::new();

    // if with_html_color {
    //     setup_html_color_regexes(&mut hm);
    //     for idx in 0..headers.len() {
    //         headers[idx] = run_regexes(&hm, &mut headers[idx]);
    //     }

    // } else if with_no_color {
    //     setup_no_color_regexes(&mut hm);
    //     for idx in 0..headers.len() {
    //         headers[idx] = run_regexes(&hm, &mut headers[idx]);
    //     }
    // }

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
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
                    Some(AnchorLocation::Url(f)) | Some(AnchorLocation::File(f)) => {
                        let extension = f.split('.').last().map(String::from);
                        match extension {
                            Some(s)
                                if ["png", "jpg", "bmp", "gif", "tiff", "jpeg"]
                                    .contains(&s.to_lowercase().as_str()) =>
                            {
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
                    Some(AnchorLocation::Url(f)) | Some(AnchorLocation::File(f)) => {
                        let extension = f.split('.').last().map(String::from);
                        match extension {
                            Some(s) if s.to_lowercase() == "svg" => {
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
                output_string.push_str(
                    &(htmlescape::encode_minimal(&format_leaf(&row.value).plain_string(100_000))
                        .replace("\n", "<br>")),
                );
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
                output_string.push_str(
                    &(htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000))
                        .replace("\n", "<br>")),
                );
            }
        }
    }

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
        output_string.push_str("</table>");
    }
    output_string.push_str("</body></html>");

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output_string).into_value(name_tag),
    )))
}

// fn setup_html_color_regexes(hash: &mut HashMap<u32, (&'static str, &'static str)>) {
//     // All the bold colors
//     hash.insert(0, (r"(?P<bb>\[1;30m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:black;font-weight:bold;'>$word</span>"));
//     hash.insert(1, (r"(?P<br>\[1;31m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:red;font-weight:bold;'>$word</span>"));
//     hash.insert(2, (r"(?P<bg>\[1;32m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:green;font-weight:bold;'>$word</span>"));
//     hash.insert(3, (r"(?P<by>\[1;33m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:yellow;font-weight:bold;'>$word</span>"));
//     hash.insert(4, (r"(?P<bu>\[1;34m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:blue;font-weight:bold;'>$word</span>"));
//     hash.insert(5, (r"(?P<bm>\[1;35m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:magenta;font-weight:bold;'>$word</span>"));
//     hash.insert(6, (r"(?P<bc>\[1;36m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:cyan;font-weight:bold;'>$word</span>"));
//     hash.insert(7, (r"(?P<bw>\[1;37m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:white;font-weight:bold;'>$word</span>"));
//     // All the normal colors
//     hash.insert(8, (r"(?P<b>\[30m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:black;'>$word</span>"));
//     hash.insert(9, (r"(?P<r>\[31m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:red;'>$word</span>"));
//     hash.insert(10, (r"(?P<g>\[32m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:green;'>$word</span>"));
//     hash.insert(11, (r"(?P<y>\[33m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:yellow;'>$word</span>"));
//     hash.insert(12, (r"(?P<u>\[34m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:blue;'>$word</span>"));
//     hash.insert(13, (r"(?P<m>\[35m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:magenta;'>$word</span>"));
//     hash.insert(14, (r"(?P<c>\[36m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:cyan;'>$word</span>"));
//     hash.insert(15, (r"(?P<w>\[37m)(?P<word>[a-z\-'!/_]+)", r"<span style='color:white;'>$word</span>"));
// }

// fn setup_no_color_regexes(hash: &mut HashMap<u32, (&'static str, &'static str)>) {
//     // All the bold colors
//     hash.insert(0, (r"(?P<bb>\[1;30m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(1, (r"(?P<br>\[1;31m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(2, (r"(?P<bg>\[1;32m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(3, (r"(?P<by>\[1;33m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(4, (r"(?P<bu>\[1;34m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(5, (r"(?P<bm>\[1;35m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(6, (r"(?P<bc>\[1;36m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(7, (r"(?P<bw>\[1;37m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     // All the normal colors
//     hash.insert(8, (r"(?P<b>\[30m)(?P<word>[a-z\-'!/_]+)",  r"$word"));
//     hash.insert(9, (r"(?P<r>\[31m)(?P<word>[a-z\-'!/_]+)",  r"$word"));
//     hash.insert(10, (r"(?P<g>\[32m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(11, (r"(?P<y>\[33m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(12, (r"(?P<u>\[34m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(13, (r"(?P<m>\[35m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(14, (r"(?P<c>\[36m)(?P<word>[a-z\-'!/_]+)", r"$word"));
//     hash.insert(15, (r"(?P<w>\[37m)(?P<word>[a-z\-'!/_]+)", r"$word"));
// }

// fn run_regexes(hash: &HashMap<u32, (&'static str, &'static str)>, contents: &String) -> String {
//     let mut working_string = contents.to_owned();
//     let hash_count:u32 = hash.len() as u32;
//     for n in 0..hash_count {
//         let value = hash.get(&n).unwrap();
//         println!("{},{}", value.0, value.1);
//         let re = Regex::new(value.0).unwrap();
//         let after = re.replace_all(&working_string, value.1).to_string();
//         working_string = after.clone();
//     }
//     working_string
// }

#[cfg(test)]
mod tests {
    use super::ToHTML;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToHTML {})
    }
}
