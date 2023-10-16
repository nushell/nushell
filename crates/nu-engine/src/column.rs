use nu_protocol::Value;
use std::collections::HashSet;

pub fn get_columns(input: &[Value]) -> Vec<String> {
    let mut set: HashSet<&String> = HashSet::new();
    let mut cols = vec![];

    for item in input {
        let Value::Record { val, .. } = item else {
            return vec![];
        };

        for col in &val.cols {
            if set.insert(col) {
                cols.push(col.to_string());
            }
        }
    }

    cols
}

// If a column doesn't exist in the input, return it.
pub fn nonexistent_column(inputs: &[String], columns: &[String]) -> Option<String> {
    let set: HashSet<String> = HashSet::from_iter(columns.iter().cloned());

    for input in inputs {
        if set.contains(input) {
            continue;
        }
        return Some(input.clone());
    }
    None
}
