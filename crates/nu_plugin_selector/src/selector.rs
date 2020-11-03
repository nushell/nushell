use nipper::Document;
use nu_errors::ShellError;
use nu_protocol::{
    value::BooleanExt, value::StrExt, value::StringExt, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::{Tag, Tagged};
// use bigdecimal::{BigDecimal, FromPrimitive};
// use sxd_document::parser;
// use sxd_xpath::{Context, Factory};

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

pub fn begin_selector_query(raw: String, query: Tagged<&str>) -> Result<Vec<Value>, ShellError> {
    execute_selector_query(raw, query.item.to_string(), query.tag())
}

fn execute_selector_query(
    input_string: String,
    query_string: String,
    tag: impl Into<Tag>,
) -> Result<Vec<Value>, ShellError> {
    let tag = tag.into();
    let mut ret = vec![];

    ret.push("Test1".to_str_value_create_tag());
    let html = r#"<div name="foo" value="bar"></div>"#;
    let document = Document::from(html);
    ret.push(document.html().to_string().to_string_value_create_tag());
    // should = "<html><head></head><body><div name="foo" value="bar"></div></body></html>"

    ret.push("Test2".to_str_value_create_tag());
    let mut input = document.select(r#"div[name="foo"]"#);
    input.set_attr("id", "input");
    input.remove_attr("name");
    ret.push(
        input
            .attr("value")
            .unwrap()
            .to_string()
            .to_string_value_create_tag(),
    );
    // Should = "bar"

    ret.push("Test3".to_str_value_create_tag());
    ret.push(input.html().to_string().to_string_value_create_tag());
    // Should = "<div value="bar" id="input"></div>"

    ret.push("Test4".to_str_value_create_tag());
    input.replace_with_html(r#"<a href="https://wisburg.com">wisburg</a><h2>xxx</h2>"#);
    ret.push(input.html().to_string().to_string_value_create_tag());
    // Should = "<div value="bar" id="input"></div>" - not sure why

    ret.push("Test5".to_str_value_create_tag());
    // println!("{}", document.html());
    ret.push(document.html().to_string().to_string_value_create_tag());
    // Should = "<html><head></head><body><a href="https://wisburg.com">wisburg</a><h2>xxx</h2></body></html>"

    ret.push("Test6".to_str_value_create_tag());
    // Another test from Nipper
    let document = Document::from(
        r#"<div class="loginContent">
    <div class="loginContentbg">
        <div class="el-dialog__wrapper login-dialog">
            <div role="dialog" aria-modal="true" aria-label="dialog"
                class="el-dialog el-dialog--center">
                <!---->
            </div>
        </div>
        <div class="el-dialog__wrapper login-dialog">
            <div role="dialog" aria-modal="true" aria-label="dialog"
                class="el-dialog el-dialog--center">
                <!---->
                <!---->
            </div>
        </div>
    </div>
</div>"#,
    );

    let mut div = document.select("div.loginContent");
    ret.push(div.is("div").to_value_create_tag());
    // Should = "Yes"

    ret.push("Test7".to_str_value_create_tag());
    // println!("|{}|", div.text().trim());
    ret.push(format!("|{}|", div.text().trim()).to_string_value_create_tag());
    ret.push(div.text().trim().to_str_value_create_tag());
    // Should = "||" i.e. return nothing

    div.remove();

    ret.push("Test8".to_str_value_create_tag());
    // println!("{}", document.html());
    ret.push(document.html().to_string().to_string_value_create_tag());

    ret.push("some value 1".to_str_value(tag));
    ret.push(input_string.to_string_value_create_tag());
    ret.push(query_string.to_string_value_create_tag());
    Ok(ret)
    // let selector = build_selector(&query_string)?;
    // let package = parser::parse(&input_string);

    // if package.is_err() {
    //     return Err(ShellError::labeled_error(
    //         "invalid xml document",
    //         "invalid xml document",
    //         tag.span,
    //     ));
    // }

    // let package = package.expect("invalid xml document");

    // let document = package.as_document();
    // let context = Context::new();

    // // leaving this here for augmentation at some point
    // // build_variables(&arguments, &mut context);
    // // build_namespaces(&arguments, &mut context);

    // let res = selector.evaluate(&context, document.root());

    // // Some xpath statements can be long, so let's truncate it with ellipsis
    // let mut key = query_string.clone();
    // if query_string.len() >= 20 {
    //     key.truncate(17);
    //     key += "...";
    // } else {
    //     key = query_string;
    // };

    // match res {
    //     Ok(r) => {
    //         let rows: Vec<Value> = match r {
    //             sxd_xpath::Value::Nodeset(ns) => ns
    //                 .into_iter()
    //                 .map(|a| {
    //                     let mut row = TaggedDictBuilder::new(Tag::unknown());
    //                     row.insert_value(&key, UntaggedValue::string(a.string_value()));
    //                     row.into_value()
    //                 })
    //                 .collect::<Vec<Value>>(),
    //             sxd_xpath::Value::Boolean(b) => {
    //                 let mut row = TaggedDictBuilder::new(Tag::unknown());
    //                 row.insert_value(&key, UntaggedValue::boolean(b));
    //                 vec![row.into_value()]
    //             }
    //             sxd_xpath::Value::Number(n) => {
    //                 let mut row = TaggedDictBuilder::new(Tag::unknown());
    //                 row.insert_value(
    //                     &key,
    //                     UntaggedValue::decimal(BigDecimal::from_f64(n).expect("error with f64"))
    //                         .into_untagged_value(),
    //                 );

    //                 vec![row.into_value()]
    //             }
    //             sxd_xpath::Value::String(s) => {
    //                 let mut row = TaggedDictBuilder::new(Tag::unknown());
    //                 row.insert_value(&key, UntaggedValue::string(s));
    //                 vec![row.into_value()]
    //             }
    //         };

    //         Ok(rows)
    //     }
    //     Err(_) => Err(ShellError::labeled_error(
    //         "selector query error",
    //         "selector query error",
    //         tag,
    //     )),
    // }
}

// fn build_selector(selector_str: &str) -> Result<sxd_xpath::XPath, ShellError> {
//     let factory = Factory::new();

//     match factory.build(selector_str) {
//         Ok(selector) => selector.ok_or_else(|| ShellError::untagged_runtime_error("invalid selector query")),
//         Err(_) => Err(ShellError::untagged_runtime_error(
//             "expected valid selector query",
//         )),
//     }
// }

#[cfg(test)]
mod tests {
    use super::begin_selector_query as query;
    // use indexmap::indexmap;
    use nu_errors::ShellError;
    use nu_source::TaggedItem;
    use nu_test_support::value::{decimal_from_float, row};

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
