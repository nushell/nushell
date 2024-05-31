use crate::Query;
use gjson::Value as gjValue;
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Record, Signature, Span, Spanned, SyntaxShape, Value};

pub struct QueryJson;

impl SimplePluginCommand for QueryJson {
    type Plugin = Query;

    fn name(&self) -> &str {
        "query json"
    }

    fn usage(&self) -> &str {
        "execute json query on json file (open --raw <file> | query json 'query string')"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("query", SyntaxShape::String, "json query")
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

        execute_json_query(call, input, query)
    }
}

pub fn execute_json_query(
    call: &EvaluatedCall,
    input: &Value,
    query: Option<Spanned<String>>,
) -> Result<Value, LabeledError> {
    let input_string = match input.coerce_str() {
        Ok(s) => s,
        Err(e) => {
            return Err(LabeledError::new("Problem with input data").with_inner(e));
        }
    };

    let query_string = match &query {
        Some(v) => &v.item,
        None => {
            return Err(LabeledError::new("Problem with input data")
                .with_label("query string missing", call.head));
        }
    };

    // Validate the json before trying to query it
    let is_valid_json = gjson::valid(&input_string);

    if !is_valid_json {
        return Err(
            LabeledError::new("Invalid JSON").with_label("this is not valid JSON", call.head)
        );
    }

    let val: gjValue = gjson::get(&input_string, query_string);

    if query_contains_modifiers(query_string) {
        let json_str = val.json();
        Ok(Value::string(json_str, call.head))
    } else {
        Ok(convert_gjson_value_to_nu_value(&val, call.head))
    }
}

fn query_contains_modifiers(query: &str) -> bool {
    // https://github.com/tidwall/gjson.rs documents 7 modifiers as of 4/19/21
    // Some of these modifiers mean we really need to output the data as a string
    // instead of tabular data. Others don't matter.

    // Output as String
    // @ugly: Remove all whitespace from a json document.
    // @pretty: Make the json document more human readable.
    query.contains("@ugly") || query.contains("@pretty")

    // Output as Tabular
    // Since it's output as tabular, which is our default, we can just ignore these
    // @reverse: Reverse an array or the members of an object.
    // @this: Returns the current element. It can be used to retrieve the root element.
    // @valid: Ensure the json document is valid.
    // @flatten: Flattens an array.
    // @join: Joins multiple objects into a single object.
}

fn convert_gjson_value_to_nu_value(v: &gjValue, span: Span) -> Value {
    match v.kind() {
        gjson::Kind::Array => {
            let mut vals = vec![];
            v.each(|_k, v| {
                vals.push(convert_gjson_value_to_nu_value(&v, span));
                true
            });

            Value::list(vals, span)
        }
        gjson::Kind::Null => Value::nothing(span),
        gjson::Kind::False => Value::bool(false, span),
        gjson::Kind::Number => {
            let str_value = v.str();
            if str_value.contains('.') {
                Value::float(v.f64(), span)
            } else {
                Value::int(v.i64(), span)
            }
        }
        gjson::Kind::String => Value::string(v.str(), span),
        gjson::Kind::True => Value::bool(true, span),
        gjson::Kind::Object => {
            let mut record = Record::new();
            v.each(|k, v| {
                record.push(k.to_string(), convert_gjson_value_to_nu_value(&v, span));
                true
            });
            Value::record(record, span)
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
