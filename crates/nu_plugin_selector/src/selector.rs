use nipper::Document;
use nu_protocol::{value::StringExt, Value};
use nu_source::{Tag, Tagged};

pub struct Selector {
    pub query: String,
    pub tag: Tag,
    pub as_html: bool,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            query: String::new(),
            tag: Tag::unknown(),
            as_html: false,
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn begin_selector_query(input: String, query: Tagged<&str>, as_html: bool) -> Vec<Value> {
    execute_selector_query(input, query.item.to_string(), query.tag(), as_html)
}

fn execute_selector_query(
    input_string: String,
    query_string: String,
    tag: impl Into<Tag>,
    as_html: bool,
) -> Vec<Value> {
    let _tag = tag.into();
    let mut ret = vec![];
    let doc = Document::from(&input_string);

    // How to internally iterate
    // doc.nip("tr.athing").iter().for_each(|athing| {
    //     let title = format!("{}", athing.select(".title a").text().to_string());
    //     let href = athing
    //         .select(".storylink")
    //         .attr("href")
    //         .unwrap()
    //         .to_string();
    //     let title_url = format!("{} - {}\n", title, href);
    //     ret.push(title_url.to_string_value_create_tag());
    // });

    if as_html {
        doc.nip(&query_string).iter().for_each(|athing| {
            ret.push(athing.html().to_string().to_string_value_create_tag());
        });
    } else {
        doc.nip(&query_string).iter().for_each(|athing| {
            ret.push(athing.text().to_string().to_string_value_create_tag());
        });
    }
    ret
}

#[cfg(test)]
mod tests {
    use nipper::Document;

    #[test]
    fn create_document_from_string() {
        let html = r#"<div name="foo" value="bar"></div>"#;
        let document = Document::from(html);
        let shouldbe =
            r#"<html><head></head><body><div name="foo" value="bar"></div></body></html>"#;

        assert_eq!(shouldbe.to_string(), document.html().to_string());
    }

    #[test]
    fn modify_html_document() {
        let html = r#"<div name="foo" value="bar"></div>"#;
        let document = Document::from(html);
        let mut input = document.select(r#"div[name="foo"]"#);
        input.set_attr("id", "input");
        input.remove_attr("name");

        let shouldbe = "bar".to_string();
        let actual = input.attr("value").unwrap().to_string();

        assert_eq!(shouldbe, actual);
    }

    // #[test]
    // fn test_hacker_news() -> Result<(), ShellError> {
    //     let html = reqwest::blocking::get("https://news.ycombinator.com")?.text()?;
    //     let document = Document::from(&html);
    //     let result = query(html, ".hnname a".to_string(), Tag::unknown());
    //     let shouldbe = Ok(vec!["Hacker News".to_str_value_create_tag()]);
    //     assert_eq!(shouldbe, result);
    //     Ok(())
    // }
}
