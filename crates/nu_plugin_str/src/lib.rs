mod nu_plugin_str;
mod strutils;

pub use strutils::Str;

#[cfg(test)]
mod tests {
    use super::Str;
    use crate::strutils::Action;
    use nu_errors::ShellError;
    use nu_protocol::{Primitive, ReturnSuccess, TaggedDictBuilder, UntaggedValue, Value};
    use nu_source::Tag;
    use nu_value_ext::ValueExt;
    use num_bigint::BigInt;

    impl Str {
        pub fn expect_action(&self, action: Action) {
            match &self.action {
                Some(set) if set == &action => {}
                Some(other) => panic!(format!("\nExpected {:#?}\n\ngot {:#?}", action, other)),
                None => panic!(format!("\nAction {:#?} not found.", action)),
            }
        }

        pub fn expect_field(&self, field: Value) {
            let field = match field.as_column_path() {
                Ok(column_path) => column_path,
                Err(reason) => panic!(format!(
                    "\nExpected {:#?} to be a ColumnPath, \n\ngot {:#?}",
                    field, reason
                )),
            };

            match &self.field {
                Some(column_path) if column_path == &field => {}
                Some(other) => panic!(format!("\nExpected {:#?} \n\ngot {:#?}", field, other)),
                None => panic!(format!("\nField {:#?} not found.", field)),
            }
        }
    }

    pub fn get_data(for_value: Value, key: &str) -> Value {
        for_value.get_data(&key.to_string()).borrow().clone()
    }

    pub fn expect_return_value_at(
        for_results: Result<Vec<Result<ReturnSuccess, ShellError>>, ShellError>,
        at: usize,
    ) -> Value {
        let return_values = for_results
            .expect("Failed! This seems to be an error getting back the results from the plugin.");

        for (idx, item) in return_values.iter().enumerate() {
            let item = match item {
                Ok(return_value) => return_value,
                Err(reason) => panic!(format!("{}", reason)),
            };

            if idx == at {
                return item.raw_value().unwrap();
            }
        }

        panic!(format!(
            "Couldn't get return value from stream at {}. (There are {} items)",
            at,
            return_values.len() - 1
        ))
    }

    pub fn int(i: impl Into<BigInt>) -> Value {
        UntaggedValue::Primitive(Primitive::Int(i.into())).into_untagged_value()
    }

    pub fn string(input: impl Into<String>) -> Value {
        UntaggedValue::string(input.into()).into_untagged_value()
    }

    pub fn structured_sample_record(key: &str, value: &str) -> Value {
        let mut record = TaggedDictBuilder::new(Tag::unknown());
        record.insert_untagged(key.clone(), UntaggedValue::string(value));
        record.into_value()
    }

    pub fn unstructured_sample_record(value: &str) -> Value {
        UntaggedValue::string(value).into_value(Tag::unknown())
    }
}
