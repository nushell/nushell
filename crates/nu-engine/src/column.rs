use nu_protocol::Value;
use std::collections::HashSet;

pub fn get_columns<'a>(input: impl IntoIterator<Item = &'a Value>) -> Vec<String> {
    let mut columns = vec![];

    for item in input {
        if let Value::Record { cols, vals: _, .. } = item {
            for col in cols {
                if !columns.contains(col) {
                    columns.push(col.to_string());
                }
            }
        } else {
            return vec![];
        }
    }

    columns
}

// If a column doesn't exist in the input, return it.
pub fn nonexistent_column(inputs: Vec<String>, columns: Vec<String>) -> Option<String> {
    let set: HashSet<String> = HashSet::from_iter(columns.iter().cloned());

    for input in &inputs {
        if set.contains(input) {
            continue;
        }
        return Some(input.clone());
    }
    None
}
