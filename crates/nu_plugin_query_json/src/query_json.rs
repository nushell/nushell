use gjson::Value as gjValue;
use nu_errors::ShellError;
use nu_protocol::{TaggedDictBuilder, UntaggedValue, Value};
use nu_source::{Tag, Tagged};

pub struct QueryJson {
    pub query: String,
    pub tag: Tag,
}

impl QueryJson {
    pub fn new() -> QueryJson {
        QueryJson {
            query: String::new(),
            tag: Tag::unknown(),
        }
    }
}

impl Default for QueryJson {
    fn default() -> Self {
        Self::new()
    }
}

pub fn begin_json_query(input: String, query: Tagged<&str>) -> Result<Vec<Value>, ShellError> {
    execute_json_query(input, query.item.to_string(), query.tag())
}

fn execute_json_query(
    input_string: String,
    query_string: String,
    tag: impl Into<Tag>,
) -> Result<Vec<Value>, ShellError> {
    let tag = tag.into();

    // Validate the json before trying to query it
    let is_valid_json = gjson::valid(&input_string);
    if !is_valid_json {
        return Err(ShellError::labeled_error(
            "invalid json",
            "invalid json",
            tag,
        ));
    }

    let mut ret: Vec<Value> = vec![];
    let val: gjValue = gjson::get(&input_string, &query_string);

    if query_contains_modifiers(&query_string) {
        let json_str = val.json();
        let json_val = Value::from(json_str);
        ret.push(json_val);
    } else {
        let gjv = convert_gjson_value_to_nu_value(&val, &tag);

        match gjv.value {
            UntaggedValue::Primitive(_) => ret.push(gjv),
            UntaggedValue::Row(_) => ret.push(gjv),
            UntaggedValue::Table(t) => {
                // Unravel the table so it's not a table inside of a table in the output
                for v in &t {
                    let c = v.clone();
                    ret.push(c)
                }
            }
            _ => (),
        }
    }

    Ok(ret)
}
fn query_contains_modifiers(query: &str) -> bool {
    // https://github.com/tidwall/gjson.rs documents 7 modifiers as of 4/19/21
    // Some of these modifiers mean we really need to output the data as a string
    // instead of tabular data. Others don't matter.

    // Output as String
    // @ugly: Remove all whitespace from a json document.
    // @pretty: Make the json document more human readable.
    query.contains("@ugly") || query.contains("@pretty")

    // Output as Tablular
    // Since it's output as tabular, which is our default, we can just ignore these
    // @reverse: Reverse an array or the members of an object.
    // @this: Returns the current element. It can be used to retrieve the root element.
    // @valid: Ensure the json document is valid.
    // @flatten: Flattens an array.
    // @join: Joins multiple objects into a single object.
}

fn convert_gjson_value_to_nu_value(v: &gjValue, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();
    let span = tag.span;

    match v.kind() {
        gjson::Kind::Array => {
            let mut values = vec![];
            v.each(|_k, v| {
                values.push(convert_gjson_value_to_nu_value(&v, &tag));
                true
            });

            UntaggedValue::Table(values).into_value(&tag)
        }
        gjson::Kind::Null => UntaggedValue::nothing().into_value(&tag),
        gjson::Kind::False => UntaggedValue::boolean(false).into_value(&tag),
        gjson::Kind::Number => {
            let str_value = v.str();
            if str_value.contains('.') {
                UntaggedValue::decimal_from_float(v.f64(), span).into_value(&tag)
            } else {
                UntaggedValue::int(v.i64()).into_value(&tag)
            }
        }
        gjson::Kind::String => UntaggedValue::string(v.str()).into_value(&tag),
        gjson::Kind::True => UntaggedValue::boolean(true).into_value(&tag),
        // I'm not sure how to test this, so it may not work
        gjson::Kind::Object => {
            // eprint!("Object: ");
            let mut collected = TaggedDictBuilder::new(&tag);
            v.each(|k, v| {
                // eprintln!("k:{} v:{}", k.str(), v.str());
                collected.insert_value(k.str(), convert_gjson_value_to_nu_value(&v, &tag));
                true
            });
            collected.into_value()
        }
    }
}

#[cfg(test)]
mod tests {
    use gjson::{valid, Value as gjValue};

    #[test]
    fn validate_string() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val = valid(json);
        assert!(val);
    }

    #[test]
    fn answer_from_get_age() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val: gjValue = gjson::get(json, "age");
        assert_eq!(val.str(), "37");
    }

    #[test]
    fn answer_from_get_children() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val: gjValue = gjson::get(json, "children");
        assert_eq!(val.str(), r#"["Sara", "Alex", "Jack"]"#);
    }

    #[test]
    fn answer_from_get_children_count() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val: gjValue = gjson::get(json, "children.#");
        assert_eq!(val.str(), "3");
    }

    #[test]
    fn answer_from_get_friends_first_name() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val: gjValue = gjson::get(json, "friends.#.first");
        assert_eq!(val.str(), r#"["James","Roger"]"#);
    }
}
