use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

#[derive(Clone)]
pub struct SubCommand;

impl Command for SubCommand {
    fn name(&self) -> &str {
        "query xml"
    }

    fn signature(&self) -> Signature {
        Signature::build("query xml")
            .required("query", SyntaxShape::String, "An XPath query")
            .category(Category::Query)
    }

    fn usage(&self) -> &str {
        "Execute an XPath query against an XML string"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let query_string: Spanned<String> = call.req(engine_state, stack, 0)?;
        let input_value = input.into_value(call.head);
        let input_span = input_value.span()?;

        if let Value::String { val: xml, span } = input_value {
            let spanned_xml = Spanned { item: xml, span };
            return Ok(execute_xpath_query(spanned_xml, query_string)?.into_pipeline_data());
        }

        Err(ShellError::PipelineMismatch(
            "string input".into(),
            call.head,
            input_span,
        ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Query an XML string",
                example: r#""<root><foo>Nu</foo><bar>Shell</bar></root>" | query xml "//foo""#,
                result: Some(Value::List {
                    vals: vec![Value::Record {
                        cols: vec!["//foo".to_string()],
                        vals: vec![Value::string("Nu".to_string(), Span::test_data())],
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Query a file containing XML",
                example: r#"open output.xml --raw | query xml "/root/results""#,
                result: None,
            },
        ]
    }
}

pub fn execute_xpath_query(
    xml: Spanned<String>,
    query_string: Spanned<String>,
) -> Result<Value, ShellError> {
    let xpath = build_xpath(&query_string.item, &query_string.span)?;

    let package = parser::parse(&xml.item);

    return match package {
        Ok(package) => {
            let document = package.as_document();
            let context = Context::new();

            // leaving this here for augmentation at some point
            // build_variables(&arguments, &mut context);
            // build_namespaces(&arguments, &mut context);
            let res = xpath.evaluate(&context, document.root());

            // Some xpath statements can be long, so let's truncate it with ellipsis
            let mut key = query_string.item.clone();
            if key.len() >= 20 {
                key.truncate(17);
                key += "...";
            }

            match res {
                Ok(r) => {
                    let mut cols: Vec<String> = vec![];
                    let mut vals: Vec<Value> = vec![];
                    let mut records: Vec<Value> = vec![];

                    match r {
                        sxd_xpath::Value::Nodeset(ns) => {
                            for n in ns.into_iter() {
                                cols.push(key.to_string());
                                vals.push(Value::string(n.string_value(), Span::test_data()));
                            }
                        }
                        sxd_xpath::Value::Boolean(b) => {
                            cols.push(key.to_string());
                            vals.push(Value::boolean(b, Span::test_data()));
                        }
                        sxd_xpath::Value::Number(n) => {
                            cols.push(key.to_string());
                            vals.push(Value::float(n, Span::test_data()));
                        }
                        sxd_xpath::Value::String(s) => {
                            cols.push(key.to_string());
                            vals.push(Value::string(s, Span::test_data()));
                        }
                    };

                    // convert the cols and vecs to a table by creating individual records
                    // for each item so we can then use a list to make a table
                    for (k, v) in cols.iter().zip(vals.iter()) {
                        records.push(Value::Record {
                            cols: vec![k.to_string()],
                            vals: vec![v.clone()],
                            span: Span::test_data(),
                        })
                    }

                    Ok(Value::List {
                        vals: records,
                        span: Span::test_data(),
                    })
                }
                Err(_) => Err(ShellError::GenericError(
                    "xpath query error".to_string(),
                    "xpath query error".to_string(),
                    Some(query_string.span),
                    None,
                    Vec::new(),
                )),
            }
        }
        Err(_) => Err(ShellError::UnsupportedInput(
            "Input is not valid XML".into(),
            xml.span,
        )),
    };
}

fn build_xpath(xpath_str: &str, span: &Span) -> Result<sxd_xpath::XPath, ShellError> {
    let factory = Factory::new();

    match factory.build(xpath_str) {
        Ok(xpath) => xpath.ok_or_else(|| {
            ShellError::GenericError(
                "invalid xpath query".to_string(),
                "invalid xpath query".to_string(),
                Some(*span),
                None,
                Vec::new(),
            )
        }),
        Err(_) => Err(ShellError::GenericError(
            "expected valid xpath query".to_string(),
            "expected valid xpath query".to_string(),
            Some(*span),
            None,
            Vec::new(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::{Span, Spanned, Value};

    #[test]
    fn test_examples() {
        use crate::test_examples;
        test_examples(SubCommand {})
    }

    #[test]
    fn position_function_in_predicate() {
        let text = Spanned {
            item: r#"<?xml version="1.0" encoding="UTF-8"?><a><b/><b/></a>"#.to_string(),
            span: Span::test_data(),
        };

        let xpath_query: Spanned<String> = Spanned {
            item: "count(//a/*[position() = 2])".to_string(),
            span: Span::test_data(),
        };

        let actual = execute_xpath_query(text, xpath_query).expect("test should not fail");
        let expected = Value::List {
            vals: vec![Value::Record {
                cols: vec!["count(//a/*[posit...".to_string()],
                vals: vec![Value::float(1.0, Span::test_data())],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn functions_implicitly_coerce_argument_types() {
        let text = Spanned {
            item: r#"<?xml version="1.0" encoding="UTF-8"?><a>true</a>"#.to_string(),
            span: Span::test_data(),
        };

        let xpath_query: Spanned<String> = Spanned {
            item: "count(//*[contains(., true)])".to_string(),
            span: Span::test_data(),
        };

        let actual = execute_xpath_query(text, xpath_query).expect("test should not fail");
        let expected = Value::List {
            vals: vec![Value::Record {
                cols: vec!["count(//*[contain...".to_string()],
                vals: vec![Value::float(1.0, Span::test_data())],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        };

        assert_eq!(actual, expected);
    }
}
