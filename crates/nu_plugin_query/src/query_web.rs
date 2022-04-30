use crate::web_tables::WebTable;
use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{Span, Value};
use scraper::{Html, Selector as ScraperSelector};

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
            as_table: Value::string("".to_string(), Span::test_data()),
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
    let query: String = match call.get_flag("query")? {
        Some(q2) => q2,
        None => "".to_string(),
    };
    let as_html = call.has_flag("as-html");
    let attribute: String = match call.get_flag("attribute")? {
        Some(a) => a,
        None => "".to_string(),
    };
    let as_table: Value = match call.get_flag("as-table")? {
        Some(v) => v,
        None => Value::nothing(head),
    };

    let inspect = call.has_flag("inspect");

    if !&query.is_empty() && ScraperSelector::parse(&query).is_err() {
        return Err(LabeledError {
            msg: "Cannot parse this query as a valid css selector".to_string(),
            label: "Parse error".to_string(),
            span: Some(head),
        });
    }

    let selector = Selector {
        query,
        as_html,
        attribute,
        as_table,
        inspect,
    };

    match input {
        Value::String { val, span } => Ok(begin_selector_query(val.to_string(), selector, *span)),
        _ => Err(LabeledError {
            label: "requires text input".to_string(),
            msg: "Expected text from pipeline".to_string(),
            span: Some(input.span()?),
        }),
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
    } else {
        match selector.attribute.is_empty() {
            true => execute_selector_query(
                input_html.as_str(),
                selector.query.as_str(),
                selector.as_html,
                selector.inspect,
                span,
            ),
            false => execute_selector_query_with_attribute(
                input_html.as_str(),
                selector.query.as_str(),
                selector.attribute.as_str(),
                selector.inspect,
                span,
            ),
        }
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
            // TODO Find a way to get the Config object here
            if let Value::String { val, .. } = x {
                cols.push(val.to_string())
            }
        }
    }

    if inspect_mode {
        eprintln!("Passed in Column Headers = {:#?}", &cols,);
    }

    let tables = match WebTable::find_by_headers(html, &cols) {
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

    Value::List { vals, span }
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

    let mut table_out = Vec::new();
    // sometimes there are tables where the first column is the headers, kind of like
    // a table has ben rotated ccw 90 degrees, in these cases all columns will be missing
    // we keep track of this with this variable so we can deal with it later
    let mut at_least_one_row_filled = false;
    // if columns are still empty, let's just make a single column table with the data
    if cols.is_empty() {
        at_least_one_row_filled = true;
        let table_with_no_empties: Vec<_> = table.iter().filter(|item| !item.is_empty()).collect();

        let mut cols = vec![];
        let mut vals = vec![];
        for row in &table_with_no_empties {
            for (counter, cell) in row.iter().enumerate() {
                cols.push(format!("column{}", counter));
                vals.push(Value::string(cell.to_string(), span))
            }
        }
        table_out.push(Value::Record { cols, vals, span })
    } else {
        for row in &table {
            let mut vals = vec![];
            let record_cols = &cols;
            for col in &cols {
                let val = row
                    .get(col)
                    .unwrap_or(&format!("Missing column: '{}'", &col))
                    .to_string();

                if !at_least_one_row_filled && val != format!("Missing column: '{}'", &col) {
                    at_least_one_row_filled = true;
                }
                vals.push(Value::string(val, span));
            }
            table_out.push(Value::Record {
                cols: record_cols.to_vec(),
                vals,
                span,
            })
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

    Value::List {
        vals: table_out,
        span,
    }
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
    Value::List { vals, span }
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
                Value::string(
                    selection
                        .text()
                        .fold("".to_string(), |acc, x| format!("{}{}", acc, x)),
                    span,
                )
            })
            .collect(),
    };

    Value::List { vals, span }
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
        let out = item.into_string("\n", &config);
        assert_eq!("[Coffee]".to_string(), out)
    }
}
