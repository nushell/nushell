use crate::{Query, web_tables::WebTable};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, Record, Signature, Span, Spanned, SyntaxShape, Value,
};
use scraper::{Html, Selector as ScraperSelector};

pub struct QueryWeb;

impl SimplePluginCommand for QueryWeb {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query web"
    }

    fn description(&self) -> &str {
        "execute selector query on html/web"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("query", SyntaxShape::String, "selector query", Some('q'))
            .switch("as-html", "return the query output as html", Some('m'))
            .named(
                "attribute",
                SyntaxShape::Any,
                "downselect based on the given attribute",
                Some('a'),
            )
            // TODO: use detailed shape when https://github.com/nushell/nushell/issues/13253 is resolved
            // .named(
            //     "attribute",
            //     SyntaxShape::OneOf(vec![
            //         SyntaxShape::List(Box::new(SyntaxShape::String)),
            //         SyntaxShape::String,
            //     ]),
            //     "downselect based on the given attribute",
            //     Some('a'),
            // )
            .named(
                "as-table",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "find table based on column header list",
                Some('t'),
            )
            .switch(
                "inspect",
                "run in inspect mode to provide more information for determining column headers",
                Some('i'),
            )
            .category(Category::Network)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        web_examples()
    }

    fn run(
        &self,
        _plugin: &Query,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        parse_selector_params(call, input)
    }
}

pub fn web_examples() -> Vec<Example<'static>> {
    vec![
        Example {
            example: "http get https://phoronix.com | query web --query 'header' | flatten",
            description: "Retrieve all `<header>` elements from phoronix.com website",
            result: None,
        },
        Example {
            example: "http get https://en.wikipedia.org/wiki/List_of_cities_in_India_by_population |
        query web --as-table [City 'Population(2011)[3]' 'Population(2001)[3][a]' 'State or unionterritory' 'Reference']",
            description: "Retrieve a html table from Wikipedia and parse it into a nushell table using table headers as guides",
            result: None
        },
        Example {
            example: "http get https://www.nushell.sh | query web --query 'h2, h2 + p' | each {str join} | chunks 2 | each {rotate --ccw tagline description} | flatten",
            description: "Pass multiple css selectors to extract several elements within single query, group the query results together and rotate them to create a table",
            result: None,
        },
        Example {
            example: "http get https://example.org | query web --query a --attribute href",
            description: "Retrieve a specific html attribute instead of the default text",
            result: None,
        },
        Example {
            example: r#"http get https://www.rust-lang.org | query web --query 'meta[property^="og:"]' --attribute [ property content ]"#,
            description: r#"Retrieve the OpenGraph properties (`<meta property="og:...">`) from a web page"#,
            result: None,
        }
    ]
}

pub struct Selector {
    pub query: Spanned<String>,
    pub as_html: bool,
    pub attribute: Value,
    pub as_table: Value,
    pub inspect: Spanned<bool>,
}

pub fn parse_selector_params(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let head = call.head;
    let query: Option<Spanned<String>> = call.get_flag("query")?;
    let as_html = call.has_flag("as-html")?;
    let attribute = call
        .get_flag("attribute")?
        .unwrap_or_else(|| Value::nothing(head));
    let as_table: Value = call
        .get_flag("as-table")?
        .unwrap_or_else(|| Value::nothing(head));

    let inspect = call.has_flag("inspect")?;
    let inspect_span = call.get_flag_span("inspect").unwrap_or(call.head);

    let selector = Selector {
        query: query.unwrap_or(Spanned {
            span: call.head,
            item: "".to_owned(),
        }),
        as_html,
        attribute,
        as_table,
        inspect: Spanned {
            item: inspect,
            span: inspect_span,
        },
    };

    let span = input.span();
    match input {
        Value::String { val, .. } => begin_selector_query(val.to_string(), selector, span),
        _ => Err(LabeledError::new("Requires text input")
            .with_label("expected text from pipeline", span)),
    }
}

fn begin_selector_query(
    input_html: String,
    selector: Selector,
    span: Span,
) -> Result<Value, LabeledError> {
    if let Value::List { .. } = selector.as_table {
        retrieve_tables(
            input_html.as_str(),
            &selector.as_table,
            selector.inspect.item,
            span,
        )
    } else if selector.attribute.is_empty() {
        execute_selector_query(
            input_html.as_str(),
            selector.query,
            selector.as_html,
            selector.inspect,
            span,
        )
    } else if let Value::List { .. } = selector.attribute {
        execute_selector_query_with_attributes(
            input_html.as_str(),
            selector.query,
            &selector.attribute,
            selector.inspect,
            span,
        )
    } else {
        execute_selector_query_with_attribute(
            input_html.as_str(),
            selector.query,
            selector.attribute.as_str().unwrap_or(""),
            selector.inspect,
            span,
        )
    }
}

pub fn retrieve_tables(
    input_string: &str,
    columns: &Value,
    inspect_mode: bool,
    span: Span,
) -> Result<Value, LabeledError> {
    let html = input_string;
    let mut cols: Vec<String> = Vec::new();
    if let Value::List { vals, .. } = &columns {
        for x in vals {
            if let Value::String { val, .. } = x {
                cols.push(val.to_string())
            }
        }
    }

    if inspect_mode {
        eprintln!("Passed in Column Headers = {:?}\n", &cols);
        eprintln!("First 2048 HTML chars = {}\n", &html[0..2047]);
    }

    let tables = match WebTable::find_by_headers(html, &cols, inspect_mode) {
        Some(t) => {
            if inspect_mode {
                eprintln!("Table Found = {:#?}", &t);
            }
            t
        }
        None => vec![WebTable::empty()],
    };

    if tables.len() == 1 {
        return Ok(retrieve_table(
            tables.into_iter().next().ok_or_else(|| {
                LabeledError::new("Cannot retrieve table")
                    .with_label("Error retrieving table.", span)
                    .with_help("No table found.")
            })?,
            columns,
            span,
        ));
    }

    let vals = tables
        .into_iter()
        .map(move |table| retrieve_table(table, columns, span))
        .collect();

    Ok(Value::list(vals, span))
}

fn retrieve_table(mut table: WebTable, columns: &Value, span: Span) -> Value {
    let mut cols: Vec<String> = Vec::new();
    if let Value::List { vals, .. } = &columns {
        for x in vals {
            // TODO Find a way to get the Config object here
            if let Value::String { val, .. } = x {
                cols.push(val.to_string())
            }
        }
    }

    if cols.is_empty() && !table.headers().is_empty() {
        for col in table.headers().keys() {
            cols.push(col.to_string());
        }
    }

    // We provided columns but the table has no headers, so we'll just make a single column table
    if !cols.is_empty() && table.headers().is_empty() {
        let mut record = Record::new();
        for col in &cols {
            record.push(
                col.clone(),
                Value::string("error: no data found (column name may be incorrect)", span),
            );
        }
        return Value::record(record, span);
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

        let mut record = Record::new();
        for row in &table_with_no_empties {
            for (counter, cell) in row.iter().enumerate() {
                record.push(format!("column{counter}"), Value::string(cell, span));
            }
        }
        table_out.push(Value::record(record, span))
    } else {
        for row in &table {
            let record = cols
                .iter()
                .map(|col| {
                    let val = row
                        .get(col)
                        .unwrap_or(&format!("Missing column: '{}'", &col))
                        .to_string();

                    if !at_least_one_row_filled && val != format!("Missing column: '{}'", &col) {
                        at_least_one_row_filled = true;
                    }
                    (col.clone(), Value::string(val, span))
                })
                .collect();
            table_out.push(Value::record(record, span))
        }
    }
    if !at_least_one_row_filled {
        let mut data2 = Vec::new();
        for x in &table.data {
            data2.push(x.join(", "));
        }
        table.data = vec![data2];
        return retrieve_table(table, columns, span);
    }
    // table_out

    Value::list(table_out, span)
}

fn execute_selector_query_with_attribute(
    input_string: &str,
    query_string: Spanned<String>,
    attribute: &str,
    inspect: Spanned<bool>,
    span: Span,
) -> Result<Value, LabeledError> {
    let doc = Html::parse_fragment(input_string);

    let vals: Vec<Value> = doc
        .select(&fallible_css(query_string, inspect)?)
        .map(|selection| {
            Value::string(
                selection.value().attr(attribute).unwrap_or("").to_string(),
                span,
            )
        })
        .collect();
    Ok(Value::list(vals, span))
}

fn execute_selector_query_with_attributes(
    input_string: &str,
    query_string: Spanned<String>,
    attributes: &Value,
    inspect: Spanned<bool>,
    span: Span,
) -> Result<Value, LabeledError> {
    let doc = Html::parse_fragment(input_string);

    let mut attrs: Vec<String> = Vec::new();
    if let Value::List { vals, .. } = &attributes {
        for x in vals {
            if let Value::String { val, .. } = x {
                attrs.push(val.to_string())
            }
        }
    }

    let vals: Vec<Value> = doc
        .select(&fallible_css(query_string, inspect)?)
        .map(|selection| {
            let mut record = Record::new();
            for attr in &attrs {
                record.push(
                    attr.to_string(),
                    Value::string(selection.value().attr(attr).unwrap_or("").to_string(), span),
                );
            }
            Value::record(record, span)
        })
        .collect();
    Ok(Value::list(vals, span))
}

fn execute_selector_query(
    input_string: &str,
    query_string: Spanned<String>,
    as_html: bool,
    inspect: Spanned<bool>,
    span: Span,
) -> Result<Value, LabeledError> {
    let doc = Html::parse_fragment(input_string);

    let vals: Vec<Value> = match as_html {
        true => doc
            .select(&fallible_css(query_string, inspect)?)
            .map(|selection| Value::string(selection.html(), span))
            .collect(),
        false => doc
            .select(&fallible_css(query_string, inspect)?)
            .map(|selection| {
                Value::list(
                    selection
                        .text()
                        .map(|text| Value::string(text, span))
                        .collect(),
                    span,
                )
            })
            .collect(),
    };

    Ok(Value::list(vals, span))
}

fn fallible_css(
    selector: Spanned<String>,
    inspect: Spanned<bool>,
) -> Result<ScraperSelector, LabeledError> {
    if inspect.item {
        ScraperSelector::parse("html").map_err(|e| {
            LabeledError::new("CSS query parse error")
                .with_label(e.to_string(), inspect.span)
                .with_help(
                    "cannot parse query `html` as a valid CSS selector, possibly an internal error",
                )
        })
    } else {
        ScraperSelector::parse(&selector.item).map_err(|e| {
            LabeledError::new("CSS query parse error")
                .with_label(e.to_string(), selector.span)
                .with_help("cannot parse query as a valid CSS selector")
        })
    }
}

pub fn css(selector: &str, inspect: bool) -> ScraperSelector {
    if inspect {
        ScraperSelector::parse("html").expect("Error unwrapping the default scraperselector")
    } else {
        ScraperSelector::parse(selector).expect("Error unwrapping scraperselector::parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_LIST: &str = r#"
         <ul>
             <li>Coffee</li>
             <li>Tea</li>
             <li>Milk</li>
         </ul>
     "#;

    const NESTED_TEXT: &str = r#"<p>Hello there, <span style="color: red;">World</span></p>"#;
    const MULTIPLE_ATTRIBUTES: &str = r#"
        <a href="https://example.org" target="_blank">Example</a>
        <a href="https://example.com" target="_self">Example</a>
    "#;

    fn null_spanned<T: ToOwned + ?Sized>(input: &T) -> Spanned<T::Owned> {
        Spanned {
            item: input.to_owned(),
            span: Span::unknown(),
        }
    }

    #[test]
    fn test_first_child_is_not_empty() {
        assert!(
            !execute_selector_query(
                SIMPLE_LIST,
                null_spanned("li:first-child"),
                false,
                null_spanned(&false),
                Span::test_data()
            )
            .unwrap()
            .is_empty()
        )
    }

    #[test]
    fn test_first_child() {
        let item = execute_selector_query(
            SIMPLE_LIST,
            null_spanned("li:first-child"),
            false,
            null_spanned(&false),
            Span::test_data(),
        )
        .unwrap();
        let config = nu_protocol::Config::default();
        let out = item.to_expanded_string("\n", &config);
        assert_eq!("[[Coffee]]".to_string(), out)
    }

    #[test]
    fn test_nested_text_nodes() {
        let item = execute_selector_query(
            NESTED_TEXT,
            null_spanned("p:first-child"),
            false,
            null_spanned(&false),
            Span::test_data(),
        )
        .unwrap();
        let out = item
            .into_list()
            .unwrap()
            .into_iter()
            .map(|matches| {
                matches
                    .into_list()
                    .unwrap()
                    .into_iter()
                    .map(|text_nodes| text_nodes.coerce_into_string().unwrap())
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();

        assert_eq!(
            out,
            vec![vec!["Hello there, ".to_string(), "World".to_string()]],
        );
    }

    #[test]
    fn test_multiple_attributes() {
        let item = execute_selector_query_with_attributes(
            MULTIPLE_ATTRIBUTES,
            null_spanned("a"),
            &Value::list(
                vec![
                    Value::string("href".to_string(), Span::unknown()),
                    Value::string("target".to_string(), Span::unknown()),
                ],
                Span::unknown(),
            ),
            null_spanned(&false),
            Span::test_data(),
        )
        .unwrap();
        let out = item
            .into_list()
            .unwrap()
            .into_iter()
            .map(|matches| {
                matches
                    .into_record()
                    .unwrap()
                    .into_iter()
                    .map(|(key, value)| (key, value.coerce_into_string().unwrap()))
                    .collect::<Vec<(String, String)>>()
            })
            .collect::<Vec<Vec<(String, String)>>>();

        assert_eq!(
            out,
            vec![
                vec![
                    ("href".to_string(), "https://example.org".to_string()),
                    ("target".to_string(), "_blank".to_string())
                ],
                vec![
                    ("href".to_string(), "https://example.com".to_string()),
                    ("target".to_string(), "_self".to_string())
                ]
            ]
        )
    }
}
