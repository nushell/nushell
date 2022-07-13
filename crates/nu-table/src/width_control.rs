use crate::textstyle::TextStyle;
use crate::StyledString;

pub(crate) fn maybe_truncate_columns(
    headers: &mut Option<Vec<StyledString>>,
    data: &mut [Vec<StyledString>],
    length: usize,
    termwidth: usize,
) {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;

    // If we have too many columns, truncate the table
    if let Some(headers) = headers {
        if max_num_of_columns < length {
            headers.truncate(max_num_of_columns);
            headers.push(StyledString::new(
                String::from("..."),
                TextStyle::basic_center(),
            ));
        }
    }

    if max_num_of_columns < length {
        for entry in data.iter_mut() {
            entry.truncate(max_num_of_columns);
            entry.push(StyledString::new(
                String::from("..."),
                TextStyle::basic_center(),
            ));
        }
    }
}
