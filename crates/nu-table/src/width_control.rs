use crate::textstyle::TextStyle;
use crate::{StyledString, TableTheme};
use std::iter::Iterator;

pub(crate) fn maybe_truncate_columns(
    headers: &mut Vec<StyledString>,
    data: &mut [Vec<StyledString>],
    termwidth: usize,
) {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;

    // If we have too many columns, truncate the table
    if max_num_of_columns < headers.len() {
        headers.truncate(max_num_of_columns);
        headers.push(StyledString::new(
            String::from("..."),
            TextStyle::basic_center(),
        ));
    }

    if max_num_of_columns < headers.len() {
        for entry in data.iter_mut() {
            entry.truncate(max_num_of_columns);
            entry.push(StyledString::new(
                String::from("..."),
                TextStyle::basic_center(),
            ));
        }
    }
}

pub(crate) fn estimate_max_column_width(
    headers: &[StyledString],
    data: &[Vec<StyledString>],
    termwidth: usize,
) -> Option<usize> {
    let max_per_column = get_max_column_widths(headers, data);

    let headers_len = headers.len();
    // Measure how big our columns need to be (accounting for separators also)
    let max_naive_column_width = (termwidth - 3 * (headers_len - 1)) / headers_len;

    let column_space = ColumnSpace::measure(&max_per_column, max_naive_column_width, headers_len);

    // This gives us the max column width
    let max_column_width = column_space.max_width(termwidth)?;

    // This width isn't quite right, as we're rounding off some of our space
    let column_space = column_space.fix_almost_column_width(
        &max_per_column,
        max_naive_column_width,
        max_column_width,
        headers_len,
    );

    // This should give us the final max column width
    let max_column_width = column_space.max_width(termwidth)?;

    Some(max_column_width)
}

pub(crate) fn fix_termwidth(termwidth: usize, theme: &TableTheme) -> Option<usize> {
    let edges_width = if theme.is_left_set && theme.is_right_set {
        3
    } else if theme.is_left_set || theme.is_right_set {
        1
    } else {
        0
    };

    if termwidth < edges_width {
        return None;
    }

    Some(termwidth - edges_width - 1)
}

fn get_max_column_widths(headers: &[StyledString], data: &[Vec<StyledString>]) -> Vec<usize> {
    use std::cmp::max;

    let mut output = vec![0; headers.len()];

    for (col, content) in headers.iter().enumerate() {
        let content = clean(&content.contents);
        let content_width = tabled::papergrid::string_width_multiline(&content);
        output[col] = max(output[col], content_width);
    }

    for row in data {
        for (col, content) in row.iter().enumerate() {
            let content = clean(&content.contents);
            let content_width = tabled::papergrid::string_width_multiline(&content);
            output[col] = max(output[col], content_width);
        }
    }

    output
}

struct ColumnSpace {
    num_overages: usize,
    underage_sum: usize,
    overage_separator_sum: usize,
}

impl ColumnSpace {
    /// Measure how much space we have once we subtract off the columns who are small enough
    fn measure(
        max_per_column: &[usize],
        max_naive_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut underage_sum = 0;
        let mut overage_separator_sum = 0;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                num_overages += 1;
                if i != (headers_len - 1) {
                    overage_separator_sum += 3;
                }
                if i == 0 {
                    overage_separator_sum += 1;
                }
            } else {
                underage_sum += column_max;
                // if column isn't last, add 3 for its separator
                if i != (headers_len - 1) {
                    underage_sum += 3;
                }
                if i == 0 {
                    underage_sum += 1;
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn fix_almost_column_width(
        self,
        max_per_column: &[usize],
        max_naive_column_width: usize,
        max_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut overage_separator_sum = 0;
        let mut underage_sum = self.underage_sum;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                if column_max <= max_column_width {
                    underage_sum += column_max;
                    // if column isn't last, add 3 for its separator
                    if i != (headers_len - 1) {
                        underage_sum += 3;
                    }
                    if i == 0 {
                        underage_sum += 1;
                    }
                } else {
                    // Column is still too large, so let's count it
                    num_overages += 1;
                    if i != (headers_len - 1) {
                        overage_separator_sum += 3;
                    }
                    if i == 0 {
                        overage_separator_sum += 1;
                    }
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn max_width(&self, termwidth: usize) -> Option<usize> {
        let ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        } = self;

        if *num_overages > 0 {
            termwidth
                .checked_sub(1)?
                .checked_sub(*underage_sum)?
                .checked_sub(*overage_separator_sum)?
                .checked_div(*num_overages)
        } else {
            Some(99999)
        }
    }
}

fn clean(input: &str) -> String {
    let input = input.replace('\r', "");

    input.replace('\t', "    ")
}
