use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{Tag, Tagged};
use bigdecimal::{BigDecimal, FromPrimitive};
use sxd_document::parser;
use sxd_xpath::{Context, Factory};

pub struct Selector {
    pub query: String,
    pub tag: Tag,
}

impl Selector {
    pub fn new() -> Selector {
        Selector {
            query: String::new(),
            tag: Tag::unknown(),
        }
    }
}

impl Default for Selector {
    fn default() -> Self {
        Self::new()
    }
}

pub fn string_to_value(raw: String, query: Tagged<&str>) -> Result<Vec<Value>, ShellError> {
    execute_selector_query(raw, query.item.to_string(), query.tag())
}

fn execute_selector_query(
    input_string: String,
    query_string: String,
    tag: impl Into<Tag>,
) -> Result<Vec<Value>, ShellError> {
    let tag = tag.into();
    let selector = build_selector(&query_string)?;

    let package = parser::parse(&input_string);

    if package.is_err() {
        return Err(ShellError::labeled_error(
            "invalid xml document",
            "invalid xml document",
            tag.span,
        ));
    }

    let package = package.expect("invalid xml document");

    let document = package.as_document();
    let context = Context::new();

    // leaving this here for augmentation at some point
    // build_variables(&arguments, &mut context);
    // build_namespaces(&arguments, &mut context);

    let res = selector.evaluate(&context, document.root());

    // Some xpath statements can be long, so let's truncate it with ellipsis
    let mut key = query_string.clone();
    if query_string.len() >= 20 {
        key.truncate(17);
        key += "...";
    } else {
        key = query_string;
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

            Ok(rows)
        }
        Err(_) => Err(ShellError::labeled_error(
            "selector query error",
            "selector query error",
            tag,
        )),
    }
}

fn build_selector(selector_str: &str) -> Result<sxd_xpath::XPath, ShellError> {
    let factory = Factory::new();

    match factory.build(selector_str) {
        Ok(selector) => selector.ok_or_else(|| ShellError::untagged_runtime_error("invalid selector query")),
        Err(_) => Err(ShellError::untagged_runtime_error(
            "expected valid selector query",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::string_to_value as query;
    use nu_errors::ShellError;
    use nu_source::TaggedItem;
    use nu_test_support::value::{decimal_from_float, row};
    use indexmap::indexmap;

    #[test]
    fn position_function_in_predicate() -> Result<(), ShellError> {
        let text = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><a><b/><b/></a>"#);
        let actual = query(text, "count(//a/*[position() = 2])".tagged_unknown())?;

        assert_eq!(
            actual[0],
            row(indexmap! { "count(//a/*[posit...".into() => decimal_from_float(1.0) })
        );

        Ok(())
    }

    #[test]
    fn functions_implicitly_coerce_argument_types() -> Result<(), ShellError> {
        let text = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><a>true</a>"#);
        let actual = query(text, "count(//*[contains(., true)])".tagged_unknown())?;

        assert_eq!(
            actual[0],
            row(indexmap! { "count(//*[contain...".into() => decimal_from_float(1.0) })
        );

        Ok(())
    }
}
