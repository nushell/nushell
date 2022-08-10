use tabled::papergrid::{records::records_info_colored::RecordsInfo, GridConfig};

use crate::TextStyle;

pub(crate) fn maybe_truncate_columns(
    data: &mut RecordsInfo<'_, TextStyle>,
    length: usize,
    termwidth: usize,
) -> bool {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;
    if max_num_of_columns == 0 {
        return true;
    }

    // If we have too many columns, truncate the table
    if max_num_of_columns < length {
        data.truncate(max_num_of_columns);
        data.push(String::from("..."), &GridConfig::default());
    }

    false
}
