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

/*
*  Check to see if any of the columns inside the input
*  does not exist in a vec of columns
*/

pub fn column_does_not_exist(inputs: Vec<String>, columns: Vec<String>) -> bool {
    let mut set = HashSet::new();
    for column in columns {
        set.insert(column);
    }

    for input in &inputs {
        if set.contains(input) {
            continue;
        }
        return true;
    }
    false
}
