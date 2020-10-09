extern crate sxd_document;
extern crate sxd_xpath;
use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tagged;
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

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
            example: r#"open wix\main.wxs | xpath '//@Name' | where $it == "README.txt" | count"#,
            result: Some(vec![UntaggedValue::int(1).into()]),
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
        let result_string = execute_xpath_query(input_string, query_string.to_string());

        match result_string {
            Some(r) => Ok(
                futures::stream::iter(r.into_iter().map(ReturnSuccess::value)).to_output_stream(),
            ),
            None => Err(ShellError::labeled_error(
                "xpath query error",
                "xpath query error",
                query.tag(),
            )),
        }
    }
}

pub fn execute_xpath_query(input_string: String, query_string: String) -> Option<Vec<Value>> {
    let xpath = build_xpath(&query_string);
    let package = parser::parse(&input_string).expect("failed to parse xml");
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
        key = key + "...";
    } else {
        key = query_string.clone();
    };

    match res {
        Ok(r) => {
            let rows: Vec<Value> = match r {
                sxd_xpath::Value::Nodeset(ns) => ns
                    .into_iter()
                    .map(|a| {
                        let mut row = TaggedDictBuilder::new(Tag::unknown());
                        row.insert_value(&key, UntaggedValue::string(a.string_value()));
                        row.into_value()
                    })
                    .collect::<Vec<Value>>(),
                sxd_xpath::Value::Boolean(b) => {
                    let mut row = TaggedDictBuilder::new(Tag::unknown());
                    row.insert_value(&key, UntaggedValue::boolean(b));
                    vec![row.into_value()]
                }
                sxd_xpath::Value::Number(n) => {
                    let mut row = TaggedDictBuilder::new(Tag::unknown());
                    row.insert_value(
                        &key,
                        UntaggedValue::decimal(BigDecimal::from_f64(n).expect("error with f64"))
                            .into_untagged_value(),
                    );

                    vec![row.into_value()]
                }
                sxd_xpath::Value::String(s) => {
                    let mut row = TaggedDictBuilder::new(Tag::unknown());
                    row.insert_value(&key, UntaggedValue::string(s));
                    vec![row.into_value()]
                }
            };

            if rows.len() > 0 {
                Some(rows)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

fn build_xpath(xpath_str: &str) -> sxd_xpath::XPath {
    let factory = Factory::new();

    factory
        .build(xpath_str)
        .unwrap_or_else(|e| panic!("Unable to compile XPath {}: {}", xpath_str, e))
        .expect("error with building the xpath factory")
}

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
