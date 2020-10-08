extern crate sxd_document;
extern crate sxd_xpath;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tagged;
// use libxml::parser::Parser;
// use libxml::xpath::Context;
use sxd_document::parser;
// use sxd_xpath::{evaluate_xpath, Value as sxdValue};
use sxd_xpath::{Context, Factory};
// use sxd_xpath::evaluate_xpath;
// use std::collections::HashMap;
// use sxd_xpath::nodeset::Node;

pub struct XPath;

#[derive(Deserialize)]
struct XPathArgs {
    query: Tagged<String>,
}

#[async_trait]
impl WholeStreamCommand for XPath {
    fn name(&self) -> &str {
        "xpath"
    }

    fn signature(&self) -> Signature {
        Signature::build("xpath").required("query", SyntaxShape::String, "xpath query")
    }

    fn usage(&self) -> &str {
        "execute xpath query on xml"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "find items with name attribute",
            example: r#"open wix/main.wxs"#,
            result: Some(vec![Value::from("\u{1b}[32m")]),
        }]
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let tag = args.call_info.name_tag.clone();
        let (XPathArgs { query }, input) = args.process(&registry).await?;

        let query_string = query.as_str();
        let input_string = input.collect_string(tag.clone()).await?.item;
        let result_string = do_xpath_sxd2(input_string, query_string.to_string());

        if let Some(output) = result_string {
            let vec_string: Vec<&str> = output.split('\n').collect();
            let vec_val = vec_string
                .iter()
                .map(|s| {
                    UntaggedValue::Primitive(Primitive::String((*s).to_string())).into_value(&tag)
                })
                .collect();
            let vals = UntaggedValue::Table(vec_val).into_value(tag);
            Ok(OutputStream::from(
                vals.table_entries()
                    .map(|v| ReturnSuccess::value(v.clone()))
                    .collect::<Vec<_>>(),
            ))
        // Ok(OutputStream::one(ReturnSuccess::value(
        //     // UntaggedValue::string(output).into_value(query.tag()),
        //     vals,
        // )))
        } else {
            Err(ShellError::labeled_error(
                "xml error",
                "xml error",
                query.tag(),
            ))
        }
    }
}

pub fn do_xpath_sxd2(input_string: String, query_string: String) -> Option<String> {
    let xpath = build_xpath(&query_string);
    let package = parser::parse(&input_string).expect("failed to parse xml");
    let document = package.as_document();
    let context = Context::new();

    // build_variables(&arguments, &mut context);
    // build_namespaces(&arguments, &mut context);

    let res = xpath.evaluate(&context, document.root());

    let re = match res.unwrap() {
        sxd_xpath::Value::Nodeset(ns) => ns
            .iter()
            .map(|a| format!("{}\n", a.string_value()))
            .collect(),
        sxd_xpath::Value::Boolean(b) => format!("{}", b),
        sxd_xpath::Value::Number(n) => format!("{}", n),
        sxd_xpath::Value::String(s) => s,
    };

    Some(re)
}

fn build_xpath(xpath_str: &str) -> sxd_xpath::XPath {
    let factory = Factory::new();

    factory
        .build(xpath_str)
        .unwrap_or_else(|e| panic!("Unable to compile XPath {}: {}", xpath_str, e))
        .unwrap()
}

// pub fn do_xpath_sxd(input_string: String, query_string: String) -> Option<String> {
//     let package = parser::parse(&input_string).expect("failed to parse xml");
//     let document = package.as_document();
//     let value = evaluate_xpath(&document, &query_string);

//     let result_string = match value {
//         Ok(r) => {
//             // let match_vec_content: String = r
//             //     .get_nodes_as_vec()
//             //     .iter()
//             //     .map(|e| format!("{}\n", e.get_content()))
//             //     .collect();
//             // // OutputStream::one(ReturnSuccess::value(match_vec_content))
//             // match_vec_content
//             r.string()
//         }
//         Err(e) => {
//             // return Err(ShellError::labeled_error_with_secondary(
//             //     "Could not parse as XML",
//             //     "input cannot be parsed as XML",
//             //     Tag::unknown(),
//             //     "value originates from here",
//             //     Tag::unknown(),
//             // ))
//             format!("Error=[{:?}]", e)
//         }
//     };

//     Some(result_string)
// }

// pub fn do_xpath_libxml2(input_string: String, query_string: String) -> Option<String> {
//     let parser = Parser::default();
//     let doc = parser.parse_string(input_string).unwrap();
//     let context = Context::new(&doc).unwrap();
//     let result = context.evaluate(&query_string);

//     let result_string = match result {
//         Ok(r) => {
//             let match_vec_content: String = r
//                 .get_nodes_as_vec()
//                 .iter()
//                 .map(|e| format!("{}\n", e.get_content()))
//                 .collect();
//             // OutputStream::one(ReturnSuccess::value(match_vec_content))
//             match_vec_content
//         }
//         Err(e) => {
//             // return Err(ShellError::labeled_error_with_secondary(
//             //     "Could not parse as XML",
//             //     "input cannot be parsed as XML",
//             //     Tag::unknown(),
//             //     "value originates from here",
//             //     Tag::unknown(),
//             // ))
//             format!("Error=[{:?}]", e)
//         }
//     };

//     Some(result_string)
// }

#[cfg(test)]
mod tests {
    use super::ShellError;
    use super::XPath;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        Ok(test_examples(XPath {})?)
    }
}
