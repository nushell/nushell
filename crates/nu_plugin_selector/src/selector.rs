use nipper::Document;
use nu_protocol::{value::StringExt, Value};
use nu_source::Tag;

pub struct Selector {
    pub query: String,
    pub tag: Tag,
    pub as_html: bool,
    pub attribute: String,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            query: String::new(),
            tag: Tag::unknown(),
            as_html: false,
            attribute: String::new(),
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn begin_selector_query(input_html: String, selector: &Selector) -> Vec<Value> {
    match selector.attribute.is_empty() {
        true => execute_selector_query(
            input_html.as_str(),
            selector.query.as_str(),
            selector.as_html,
        ),
        false => execute_selector_query_with_attribute(
            input_html.as_str(),
            selector.query.as_str(),
            selector.attribute.as_str(),
        ),
    }
}

fn execute_selector_query_with_attribute(
    input_string: &str,
    query_string: &str,
    attribute: &str,
) -> Vec<Value> {
    let doc = Document::from(input_string);

    doc.select(query_string)
        .iter()
        .map(|selection| {
            selection
                .attr_or(attribute, "")
                .to_string()
                .to_string_value_create_tag()
        })
        .collect()
}

fn execute_selector_query(input_string: &str, query_string: &str, as_html: bool) -> Vec<Value> {
    let doc = Document::from(input_string);

    match as_html {
        true => doc
            .select(query_string)
            .iter()
            .map(|selection| selection.html().to_string().to_string_value_create_tag())
            .collect(),
        false => doc
            .select(query_string)
            .iter()
            .map(|selection| selection.text().to_string().to_string_value_create_tag())
            .collect(),
    }
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
