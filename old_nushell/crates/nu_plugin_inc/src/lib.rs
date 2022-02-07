mod inc;
mod nu;

pub use inc::Inc;

#[cfg(test)]
mod tests {
    use super::Inc;
    use crate::inc::Action;
    use nu_protocol::Value;
    use nu_value_ext::ValueExt;

    impl Inc {
        pub fn expect_action(&self, action: Action) {
            match &self.action {
                Some(set) if set == &action => {}
                Some(_) => panic!("\nUnexpected action"),
                None => panic!("\nAction not found."),
            }
        }

        pub fn expect_field(&self, field: Value) {
            let field = match field.as_column_path() {
                Ok(column_path) => column_path,
                Err(_) => panic!("\nExpected a ColumnPath",),
            };

            match &self.field {
                Some(column_path) if column_path == &field => {}
                Some(_) => panic!("\nUnexpected field."),
                None => panic!("\nField not found."),
            }
        }
    }
}
