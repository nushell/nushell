use ahash::HashSet;
use nu_protocol::Value;

pub fn get_columns(input: &[Value]) -> Vec<String> {
    let mut columns = vec![];
    for item in input {
        let Value::Record { cols, .. } = item else {
            return vec![];
        };

        for col in cols {
            if !columns.contains(col) {
                columns.push(col.to_string());
            }
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
