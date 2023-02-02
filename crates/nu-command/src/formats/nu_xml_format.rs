use nu_protocol::Value;

pub const COLUMN_TAG_NAME: &str = "tag";
pub const COLUMN_ATTRS_NAME: &str = "attributes";
pub const COLUMN_CONTENT_NAME: &str = "content";

/// Check if this value `record<tag, attributes, content>`, if tag is string, then attributes is
/// [Value::Record] and content is [Value::List] . If tag is [Value::Nothing] than this is a
/// string entry and attributes is also [Value::Nothing] and content is [Value::String]
pub fn is_valid_node(val: &Value) -> bool {
    check_is_valid_node(val).is_some()
}

fn check_is_valid_node(val: &Value) -> Option<()> {
    match val {
        Value::Record { cols, .. } => {
            if !(cols.len() == 3) {
                None
            } else {
                let tag = val.get_data_by_key(COLUMN_TAG_NAME)?;
                let attrs = val.get_data_by_key(COLUMN_ATTRS_NAME)?;
                let content = val.get_data_by_key(COLUMN_CONTENT_NAME)?;

                match (tag, attrs, content) {
                    (Value::Nothing{..}, Value::Nothing{..}, Value::String{..}) => Some(()),
                    (Value::String{..}, Value::Record{..}, Value::List{..}) => Some(()),
                    _ => None,
                }
            }
        },
        _ => None,
    }
}