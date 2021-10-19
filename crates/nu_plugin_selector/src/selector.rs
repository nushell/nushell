use crate::Table;
use nipper::Document;
use nu_protocol::{value::StringExt, Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub struct Selector {
    pub query: String,
    pub tag: Tag,
    pub as_html: bool,
    pub attribute: String,
    pub as_table: Value,
    pub inspect: bool,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            query: String::new(),
            tag: Tag::unknown(),
            as_html: false,
            attribute: String::new(),
            as_table: Value::new(
                UntaggedValue::Primitive(Primitive::String("".to_string())),
                Tag::unknown(),
            ),
            inspect: false,
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn begin_selector_query(input_html: String, selector: &Selector) -> Vec<Value> {
    if !selector.as_table.value.is_string() {
        retrieve_tables(input_html.as_str(), &selector.as_table, selector.inspect)
    } else {
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
}

pub fn retrieve_tables(input_string: &str, columns: &Value, inspect_mode: bool) -> Vec<Value> {
    let html = input_string;
    let mut cols = Vec::new();
    if let UntaggedValue::Table(t) = &columns.value {
        for x in t {
            cols.push(x.convert_to_string());
        }
    }

    if inspect_mode {
        eprintln!("Passed in Column Headers = {:#?}", &cols,);
    }

    let tables = match Table::find_by_headers(html, &cols) {
        Some(t) => {
            if inspect_mode {
                eprintln!("Table Found = {:#?}", &t);
            }
            t
        }
        None => vec![Table::empty()],
    };
    if tables.len() == 1 {
        return retrieve_table(
            tables
                .into_iter()
                .next()
                .expect("This should never trigger"),
            columns,
        );
    }
    tables
        .into_iter()
        .map(move |table| {
            UntaggedValue::Table(retrieve_table(table, columns)).into_value(Tag::unknown())
        })
        .collect()
}

fn retrieve_table(mut table: Table, columns: &Value) -> Vec<Value> {
    let mut cols = Vec::new();
    if let UntaggedValue::Table(t) = &columns.value {
        for x in t {
            cols.push(x.convert_to_string());
        }
    }

    if cols.is_empty() && !table.headers().is_empty() {
        for col in table.headers().keys() {
            cols.push(col.to_string());
        }
    }

    let mut table_out = Vec::new();
    // sometimes there are tables where the first column is the headers, kind of like
    // a table has ben rotated ccw 90 degrees, in these cases all columns will be missing
    // we keep track of this with this variable so we can deal with it later
    let mut at_least_one_row_filled = false;
    // if columns are still empty, let's just make a single column table with the data
    if cols.is_empty() {
        at_least_one_row_filled = true;
        let table_with_no_empties: Vec<_> = table.iter().filter(|item| !item.is_empty()).collect();

        for row in &table_with_no_empties {
            let mut dict = TaggedDictBuilder::new(Tag::unknown());
            for (counter, cell) in row.iter().enumerate() {
                let col_name = format!("Column{}", counter);
                dict.insert_value(
                    col_name,
                    UntaggedValue::Primitive(Primitive::String(cell.to_string()))
                        .into_value(Tag::unknown()),
                );
            }
            table_out.push(dict.into_value());
        }
    } else {
        for row in &table {
            let mut dict = TaggedDictBuilder::new(Tag::unknown());
            // eprintln!("row={:?}", &row);
            for col in &cols {
                //eprintln!("col={:?}", &col);
                let key = col.to_string();
                let val = row
                    .get(col)
                    .unwrap_or(&format!("Missing column: '{}'", &col))
                    .to_string();
                if !at_least_one_row_filled && val != format!("Missing column: '{}'", &col) {
                    at_least_one_row_filled = true;
                }
                dict.insert_value(
                    key,
                    UntaggedValue::Primitive(Primitive::String(val)).into_value(Tag::unknown()),
                );
            }
            table_out.push(dict.into_value());
        }
    }
    if !at_least_one_row_filled {
        let mut data2 = Vec::new();
        for x in &table.data {
            data2.push(x.join(", "));
        }
        table.data = vec![data2];
        return retrieve_table(table, columns);
    }
    table_out
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
