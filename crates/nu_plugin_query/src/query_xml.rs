use crate::Query;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, LabeledError, Record, Signature, Span, Spanned, SyntaxShape, Value, record,
};
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

pub struct QueryXml;

impl SimplePluginCommand for QueryXml {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query xml"
    }

    fn description(&self) -> &str {
        "execute xpath query on xml"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("query", SyntaxShape::String, "xpath query")
            .named(
                "namespaces",
                SyntaxShape::Record(vec![]),
                "map of prefixes to namespace URIs",
                Some('n'),
            )
            .category(Category::Filters)
    }

    fn run(
        &self,
        _plugin: &Query,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: &Value,
    ) -> Result<Value, LabeledError> {
        let query: Option<Spanned<String>> = call.opt(0)?;
        let namespaces: Option<Record> = call.get_flag::<Record>("namespaces")?;

        execute_xpath_query(call, input, query, namespaces)
    }
}

pub fn execute_xpath_query(
    call: &EvaluatedCall,
    input: &Value,
    query: Option<Spanned<String>>,
    namespaces: Option<Record>,
) -> Result<Value, LabeledError> {
    let (query_string, span) = match &query {
        Some(v) => (&v.item, v.span),
        None => {
            return Err(
                LabeledError::new("problem with input data").with_label("query missing", call.head)
            );
        }
    };

    let xpath = build_xpath(query_string, span)?;
    let input_string = input.coerce_str()?;
    let package = parser::parse(&input_string);

    if let Err(err) = package {
        return Err(
            LabeledError::new("Invalid XML document").with_label(err.to_string(), input.span())
        );
    }

    let package = package.expect("invalid xml document");

    let document = package.as_document();
    let mut context = Context::new();

    for (prefix, uri) in namespaces.unwrap_or_default().into_iter() {
        context.set_namespace(prefix.as_str(), uri.into_string()?.as_str());
    }

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
                    for n in ns.document_order() {
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
        Err(err) => {
            Err(LabeledError::new("xpath query error").with_label(err.to_string(), call.head))
        }
    }
}

fn build_xpath(xpath_str: &str, span: Span) -> Result<sxd_xpath::XPath, LabeledError> {
    let factory = Factory::new();

    match factory.build(xpath_str) {
        Ok(xpath) => xpath.ok_or_else(|| {
            LabeledError::new("invalid xpath query").with_label("the query must not be empty", span)
        }),
        Err(err) => Err(LabeledError::new("invalid xpath query").with_label(err.to_string(), span)),
    }
}

#[cfg(test)]
mod tests {
    use super::execute_xpath_query as query;
    use nu_plugin::EvaluatedCall;
    use nu_protocol::{Span, Spanned, Value, record};

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

        let actual = query(&call, &text, Some(spanned_str), None).expect("test should not fail");

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

        let actual = query(&call, &text, Some(spanned_str), None).expect("test should not fail");

        let expected = Value::list(
            vec![Value::test_record(record! {
                "count(//*[contain..." => Value::test_float(1.0),
            })],
            Span::test_data(),
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn namespaces_are_used() {
        let call = EvaluatedCall {
            head: Span::test_data(),
            positional: vec![],
            named: vec![],
        };

        // document uses `dp` ("document prefix") as a prefix
        let text = Value::string(
            r#"<?xml version="1.0" encoding="UTF-8"?><a xmlns:dp="http://example.com/ns"><dp:b>yay</dp:b></a>"#,
            Span::test_data(),
        );

        // but query uses `qp` ("query prefix") as a prefix
        let namespaces = record! {
            "qp" => Value::string("http://example.com/ns", Span::test_data()),
        };

        let spanned_str: Spanned<String> = Spanned {
            item: "//qp:b/text()".to_string(),
            span: Span::test_data(),
        };

        let actual =
            query(&call, &text, Some(spanned_str), Some(namespaces)).expect("test should not fail");

        let expected = Value::list(
            vec![Value::test_record(record! {
                "//qp:b/text()" => Value::string("yay", Span::test_data()),
            })],
            Span::test_data(),
        );

        // and yet it should work regardless
        assert_eq!(actual, expected);
    }
}
