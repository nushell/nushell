use indexmap::IndexSet;
use nu_protocol::Value;
use std::collections::HashSet;

pub fn get_columns(input: &[Value]) -> Vec<String> {
    let mut set: IndexSet<&String> = IndexSet::new();

    for item in input {
        let Value::Record { val, .. } = item else {
            return vec![];
        };

        set.extend(&val.cols);
    }

    set.into_iter().map(|x| x.to_string()).collect()
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
