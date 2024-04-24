use crate::{web_tables::WebTable, Query};
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

    fn usage(&self) -> &str {
        "execute selector query on html/web"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("query", SyntaxShape::String, "selector query", Some('q'))
            .switch("as-html", "return the query output as html", Some('m'))
            .named(
                "attribute",
                SyntaxShape::String,
                "downselect based on the given attribute",
                Some('a'),
            )
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

    fn examples(&self) -> Vec<Example> {
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
        query web --as-table [City 'Population(2011)[3]' 'Population(2001)[3][a]' 'State or unionterritory' 'Ref']",
            description: "Retrieve a html table from Wikipedia and parse it into a nushell table using table headers as guides",
            result: None
        },
        Example {
            example: "http get https://www.nushell.sh | query web --query 'h2, h2 + p' | each {str join} | group 2 | each {rotate --ccw tagline description} | flatten",
            description: "Pass multiple css selectors to extract several elements within single query, group the query results together and rotate them to create a table",
            result: None,
        },
        Example {
            example: "http get https://example.org | query web --query a --attribute href",
            description: "Retrieve a specific html attribute instead of the default text",
            result: None,
        }
    ]
}

pub struct Selector {
    pub query: String,
    pub as_html: bool,
    pub attribute: String,
    pub as_table: Value,
    pub inspect: bool,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            query: String::new(),
            as_html: false,
            attribute: String::new(),
            as_table: Value::string("".to_string(), Span::unknown()),
            inspect: false,
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_selector_params(call: &EvaluatedCall, input: &Value) -> Result<Value, LabeledError> {
    let head = call.head;
    let query: Option<Spanned<String>> = call.get_flag("query")?;
    let as_html = call.has_flag("as-html")?;
    let attribute = call.get_flag("attribute")?.unwrap_or_default();
    let as_table: Value = call
        .get_flag("as-table")?
        .unwrap_or_else(|| Value::nothing(head));

    let inspect = call.has_flag("inspect")?;

    if let Some(query) = &query {
        if let Err(err) = ScraperSelector::parse(&query.item) {
            return Err(LabeledError::new("CSS query parse error")
                .with_label(err.to_string(), query.span)
                .with_help("cannot parse this query as a valid CSS selector"));
        }
    } else {
        return Err(
            LabeledError::new("Missing query argument").with_label("add --query here", call.head)
        );
    }

    let selector = Selector {
        query: query.map(|q| q.item).unwrap_or_default(),
        as_html,
        attribute,
        as_table,
        inspect,
    };

    let span = input.span();
    match input {
        Value::String { val, .. } => Ok(begin_selector_query(val.to_string(), selector, span)),
        _ => Err(LabeledError::new("Requires text input")
            .with_label("expected text from pipeline", span)),
    }
}

fn begin_selector_query(input_html: String, selector: Selector, span: Span) -> Value {
    if let Value::List { .. } = selector.as_table {
        return retrieve_tables(
            input_html.as_str(),
            &selector.as_table,
            selector.inspect,
            span,
        );
    } else if selector.attribute.is_empty() {
        execute_selector_query(
            input_html.as_str(),
            selector.query.as_str(),
            selector.as_html,
            selector.inspect,
            span,
        )
    } else {
        execute_selector_query_with_attribute(
            input_html.as_str(),
            selector.query.as_str(),
            selector.attribute.as_str(),
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
) -> Value {
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
        return retrieve_table(
            tables.into_iter().next().expect("Error retrieving table"),
            columns,
            span,
        );
    }

    let vals = tables
        .into_iter()
        .map(move |table| retrieve_table(table, columns, span))
        .collect();

    Value::list(vals, span)
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
    query_string: &str,
    attribute: &str,
    inspect: bool,
    span: Span,
) -> Value {
    let doc = Html::parse_fragment(input_string);

    let vals: Vec<Value> = doc
        .select(&css(query_string, inspect))
        .map(|selection| {
            Value::string(
                selection.value().attr(attribute).unwrap_or("").to_string(),
                span,
            )
        })
        .collect();
    Value::list(vals, span)
}

fn execute_selector_query(
    input_string: &str,
    query_string: &str,
    as_html: bool,
    inspect: bool,
    span: Span,
) -> Value {
    let doc = Html::parse_fragment(input_string);

    let vals: Vec<Value> = match as_html {
        true => doc
            .select(&css(query_string, inspect))
            .map(|selection| Value::string(selection.html(), span))
            .collect(),
        false => doc
            .select(&css(query_string, inspect))
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

    Value::list(vals, span)
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

    #[test]
    fn test_first_child_is_not_empty() {
        assert!(!execute_selector_query(
            SIMPLE_LIST,
            "li:first-child",
            false,
            false,
            Span::test_data()
        )
        .is_empty())
    }

    #[test]
    fn test_first_child() {
        let item = execute_selector_query(
            SIMPLE_LIST,
            "li:first-child",
            false,
            false,
            Span::test_data(),
        );
        let config = nu_protocol::Config::default();
        let out = item.to_expanded_string("\n", &config);
        assert_eq!("[[Coffee]]".to_string(), out)
    }

    #[test]
    fn test_nested_text_nodes() {
        let item = execute_selector_query(
            NESTED_TEXT,
            "p:first-child",
            false,
            false,
            Span::test_data(),
        );
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
}
