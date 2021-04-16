use gjson::Value as gjValue;
use nu_protocol::Value;
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

pub fn begin_json_query(input: String, query: Tagged<&str>) -> Vec<Value> {
    execute_json_query(input, query.item.to_string(), query.tag())
}

fn execute_json_query(
    input_string: String,
    query_string: String,
    tag: impl Into<Tag>,
) -> Vec<Value> {
    let _tag = tag.into();
    let mut ret: Vec<Value> = vec![];
    let val: gjValue = gjson::get(input_string.as_str(), &query_string);

    let json_str = val.json();
    let json_val = Value::from(json_str);
    ret.push(json_val);

    ret
}

#[cfg(test)]
mod tests {
    use gjson::{valid, Value as gjValue};

    #[test]
    fn validate_string() {
        let json = r#"{ "name": { "first": "Tom", "last": "Anderson" }, "age": 37, "children": ["Sara", "Alex", "Jack"], "friends": [ { "first": "James", "last": "Murphy" }, { "first": "Roger", "last": "Craig" } ] }"#;
        let val = valid(json);
        assert_eq!(val, true);
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
