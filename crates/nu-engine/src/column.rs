use nu_protocol::Value;

pub fn get_columns(input: &[Value]) -> Vec<String> {
    let mut columns = vec![];

    for item in input {
        if let Value::Record { cols, vals: _, .. } = item {
            for col in cols {
                if !columns.contains(col) {
                    columns.push(col.to_string());
                }
            }
        }
    }

    columns
}
