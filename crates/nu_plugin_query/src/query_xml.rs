use nu_plugin::{EvaluatedCall, LabeledError};
use nu_protocol::{record, Record, Span, Spanned, Value};
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

pub fn execute_xpath_query(
    _name: &str,
    call: &EvaluatedCall,
    input: &Value,
    query: Option<Spanned<String>>,
) -> Result<Value, LabeledError> {
    let (query_string, span) = match &query {
        Some(v) => (&v.item, v.span),
        None => {
            return Err(LabeledError {
                msg: "problem with input data".to_string(),
                label: "problem with input data".to_string(),
                span: Some(call.head),
            })
        }
    };

    let xpath = build_xpath(query_string, span)?;
    let input_string = input.as_string()?;
    let package = parser::parse(&input_string);

    if package.is_err() {
        return Err(LabeledError {
            label: "invalid xml document".to_string(),
            msg: "invalid xml document".to_string(),
            span: Some(call.head),
        });
    }

    let package = package.expect("invalid xml document");

    let document = package.as_document();
    let context = Context::new();

    // leaving this here for augmentation at some point
    // build_variables(&arguments, &mut context);
    // build_namespaces(&arguments, &mut context);
    let res = xpath.evaluate(&context, document.root());

    // Some xpath statements can be long, so let's truncate it with ellipsis
    let mut key = query_string.clone();
    if query_string.len() >= 20 {
        key.truncate(17);
        key += "...";
    } else {
        key = query_string.to_string();
    };

    match res {
        Ok(r) => {
            let mut record = Record::new();
            let mut records: Vec<Value> = vec![];

            match r {
                sxd_xpath::Value::Nodeset(ns) => {
                    for n in ns.into_iter() {
                        record.push(key.clone(), Value::string(n.string_value(), call.head));
                    }
                }
                sxd_xpath::Value::Boolean(b) => {
                    record.push(key, Value::bool(b, call.head));
                }
                sxd_xpath::Value::Number(n) => {
                    record.push(key, Value::float(n, call.head));
                }
                sxd_xpath::Value::String(s) => {
                    record.push(key, Value::string(s, call.head));
                }
            };

            // convert the cols and vecs to a table by creating individual records
            // for each item so we can then use a list to make a table
            for (k, v) in record {
                records.push(Value::record(record! { k => v }, call.head))
            }

            Ok(Value::list(records, call.head))
        }
        Err(_) => Err(LabeledError {
            label: "xpath query error".to_string(),
            msg: "xpath query error".to_string(),
            span: Some(call.head),
        }),
    }
}

fn build_xpath(xpath_str: &str, span: Span) -> Result<sxd_xpath::XPath, LabeledError> {
    let factory = Factory::new();

    if let Ok(xpath) = factory.build(xpath_str) {
        xpath.ok_or_else(|| LabeledError {
            label: "invalid xpath query".to_string(),
            msg: "invalid xpath query".to_string(),
            span: Some(span),
        })
    } else {
        Err(LabeledError {
            label: "expected valid xpath query".to_string(),
            msg: "expected valid xpath query".to_string(),
            span: Some(span),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::execute_xpath_query as query;
    use nu_plugin::EvaluatedCall;
    use nu_protocol::{record, Span, Spanned, Value};

    #[test]
    fn position_function_in_predicate() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a><b/><b/></a>"#,
            Span::test_data(),
        );

        let spanned_str: Spanned<String> = Spanned {
            item: "count(//a/*[position() = 2])".to_string(),
            span: Span::test_data(),
        };

        let actual = query("", &call, &text, Some(spanned_str)).expect("test should not fail");
        let expected = Value::list(
            vec![Value::test_record(record! {
                "count(//a/*[posit..." => Value::test_float(1.0),
            })],
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn functions_implicitly_coerce_argument_types() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a>true</a>"#,
            Span::test_data(),
        );

        let spanned_str: Spanned<String> = Spanned {
            item: "count(//*[contains(., true)])".to_string(),
            span: Span::test_data(),
        };

        let actual = query("", &call, &text, Some(spanned_str)).expect("test should not fail");
        let expected = Value::list(
            vec![Value::test_record(record! {
                "count(//*[contain..." => Value::test_float(1.0),
            })],
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }
}
