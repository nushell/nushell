use nu_protocol::Value;
use std::collections::HashSet;

pub fn get_columns(input: &[Value]) -> Vec<String> {
    let mut columns = HashSet::new();
    for item in input {
        let Value::Record { val, .. } = item else {
            return vec![];
        };

        for col in val.columns() {
            columns.insert(col.to_string());
        }
    }

    columns.into_iter().collect()
}

// If a column doesn't exist in the input, return it.
pub fn nonexistent_column<'a, I>(inputs: &[String], columns: I) -> Option<String>
where
    I: IntoIterator<Item = &'a String>,
{
    let set: HashSet<&String> = HashSet::from_iter(columns);

    for input in inputs {
        if set.contains(input) {
            continue;
        }
        return Some(input.clone());
    }
    None
}
