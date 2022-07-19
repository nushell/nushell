pub(crate) fn maybe_truncate_columns(
    headers: &mut Option<Vec<String>>,
    data: &mut [Vec<String>],
    length: usize,
    termwidth: usize,
) -> bool {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;
    if max_num_of_columns == 0 {
        return true;
    }

    // If we have too many columns, truncate the table
    if let Some(headers) = headers {
        if max_num_of_columns < length {
            headers.truncate(max_num_of_columns);
            headers.push(String::from("..."));
        }
    }

    if max_num_of_columns < length {
        for entry in data.iter_mut() {
            entry.truncate(max_num_of_columns);
            entry.push(String::from("..."));
        }
    }

    false
}
