use crate::styled_string::StyledString;
use crate::table_theme::TableTheme;
use crate::text_style::TextStyle;
use crate::wrap::{
    column_width, split_sublines, wrap, Alignment, Subline, WrappedCell, WrappedLine,
};
use ansi_term::Style;
use std::collections::HashMap;

enum SeparatorPosition {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug)]
pub struct Table {
    pub headers: Vec<StyledString>,
    pub data: Vec<Vec<StyledString>>,
    pub theme: TableTheme,
}

impl Table {
    pub fn new(
        headers: Vec<StyledString>,
        data: Vec<Vec<StyledString>>,
        theme: TableTheme,
    ) -> Table {
        Table {
            headers,
            data,
            theme,
        }
    }
}

#[derive(Debug)]
pub struct ProcessedTable<'a> {
    pub headers: Vec<ProcessedCell<'a>>,
    pub data: Vec<Vec<ProcessedCell<'a>>>,
    pub theme: TableTheme,
}

#[derive(Debug)]
pub struct ProcessedCell<'a> {
    pub contents: Vec<Vec<Subline<'a>>>,
    pub style: TextStyle,
}

#[derive(Debug)]
pub struct WrappedTable {
    pub column_widths: Vec<usize>,
    pub headers: Vec<WrappedCell>,
    pub data: Vec<Vec<WrappedCell>>,
    pub theme: TableTheme,
}

impl WrappedTable {
    fn print_separator(
        &self,
        separator_position: SeparatorPosition,
        color_hm: &HashMap<String, Style>,
    ) {
        let column_count = self.column_widths.len();
        let mut output = String::new();
        let sep_color = color_hm
            .get("separator_color")
            .unwrap_or(&Style::default())
            .to_owned();

        match separator_position {
            SeparatorPosition::Top => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.top_left.to_string())
                                .to_string(),
                        );
                    }

                    for _ in 0..*column.1 {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.top_horizontal.to_string())
                                .to_string(),
                        );
                    }

                    output.push_str(
                        &sep_color
                            .paint(&self.theme.top_horizontal.to_string())
                            .to_string(),
                    );
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.top_horizontal.to_string())
                            .to_string(),
                    );
                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push_str(
                                &sep_color
                                    .paint(&self.theme.top_right.to_string())
                                    .to_string(),
                            );
                        }
                    } else {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.top_center.to_string())
                                .to_string(),
                        );
                    }
                }
            }
            SeparatorPosition::Middle => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.middle_left.to_string())
                                .to_string(),
                        );
                    }

                    for _ in 0..*column.1 {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.middle_horizontal.to_string())
                                .to_string(),
                        );
                    }

                    output.push_str(
                        &sep_color
                            .paint(&self.theme.middle_horizontal.to_string())
                            .to_string(),
                    );
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.middle_horizontal.to_string())
                            .to_string(),
                    );

                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push_str(
                                &sep_color
                                    .paint(&self.theme.middle_right.to_string())
                                    .to_string(),
                            );
                        }
                    } else {
                        output
                            .push_str(&sep_color.paint(&self.theme.center.to_string()).to_string());
                    }
                }
            }
            SeparatorPosition::Bottom => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.bottom_left.to_string())
                                .to_string(),
                        );
                    }
                    for _ in 0..*column.1 {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.bottom_horizontal.to_string())
                                .to_string(),
                        );
                    }
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.bottom_horizontal.to_string())
                            .to_string(),
                    );
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.bottom_horizontal.to_string())
                            .to_string(),
                    );

                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push_str(
                                &sep_color
                                    .paint(&self.theme.bottom_right.to_string())
                                    .to_string(),
                            );
                        }
                    } else {
                        output.push_str(
                            &sep_color
                                .paint(&self.theme.bottom_center.to_string())
                                .to_string(),
                        );
                    }
                }
            }
        }

        println!("{}", output);
    }

    fn print_cell_contents(&self, cells: &[WrappedCell], color_hm: &HashMap<String, Style>) {
        let sep_color = color_hm
            .get("separator_color")
            .unwrap_or(&Style::default())
            .to_owned();

        for current_line in 0.. {
            let mut lines_printed = 0;

            let mut output = String::new();
            if self.theme.print_left_border {
                output.push_str(
                    &sep_color
                        .paint(&self.theme.left_vertical.to_string())
                        .to_string(),
                );
            }

            for column in cells.iter().enumerate() {
                if let Some(line) = (column.1).lines.get(current_line) {
                    let remainder = self.column_widths[column.0] - line.width;
                    output.push(' ');

                    match column.1.style.alignment {
                        Alignment::Left => {
                            if let Some(color) = column.1.style.color_style {
                                output.push_str(&color.paint(&line.line).to_string());
                            } else {
                                output.push_str(&line.line);
                            }
                            for _ in 0..remainder {
                                output.push(' ');
                            }
                        }
                        Alignment::Center => {
                            for _ in 0..remainder / 2 {
                                output.push(' ');
                            }
                            if let Some(color) = column.1.style.color_style {
                                output.push_str(&color.paint(&line.line).to_string());
                            } else {
                                output.push_str(&line.line);
                            }
                            for _ in 0..(remainder / 2 + remainder % 2) {
                                output.push(' ');
                            }
                        }
                        Alignment::Right => {
                            for _ in 0..remainder {
                                output.push(' ');
                            }
                            if let Some(color) = column.1.style.color_style {
                                output.push_str(&color.paint(&line.line).to_string());
                            } else {
                                output.push_str(&line.line);
                            }
                        }
                    }
                    output.push(' ');
                    lines_printed += 1;
                } else {
                    for _ in 0..self.column_widths[column.0] + 2 {
                        output.push(' ');
                    }
                }
                if column.0 < cells.len() - 1 {
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.center_vertical.to_string())
                            .to_string(),
                    );
                } else if self.theme.print_right_border {
                    output.push_str(
                        &sep_color
                            .paint(&self.theme.right_vertical.to_string())
                            .to_string(),
                    );
                }
            }
            if lines_printed == 0 {
                break;
            } else {
                println!("{}", output);
            }
        }
    }

    fn print_table(&self, color_hm: &HashMap<String, Style>) {
        #[cfg(windows)]
        {
            let _ = ansi_term::enable_ansi_support();
        }

        if self.data.is_empty() {
            return;
        }

        if self.theme.print_top_border {
            self.print_separator(SeparatorPosition::Top, &color_hm);
        }

        let skip_headers = (self.headers.len() == 2 && self.headers[1].max_width == 0)
            || (self.headers.len() == 1 && self.headers[0].max_width == 0);

        if !self.headers.is_empty() && !skip_headers {
            self.print_cell_contents(&self.headers, &color_hm);
        }

        let mut first_row = true;

        for row in &self.data {
            if !first_row {
                if self.theme.separate_rows {
                    self.print_separator(SeparatorPosition::Middle, &color_hm);
                }
            } else {
                first_row = false;

                if self.theme.separate_header && !self.headers.is_empty() && !skip_headers {
                    self.print_separator(SeparatorPosition::Middle, &color_hm);
                }
            }

            self.print_cell_contents(row, &color_hm);
        }

        if self.theme.print_bottom_border {
            self.print_separator(SeparatorPosition::Bottom, &color_hm);
        }
    }
}

fn process_table(table: &Table) -> ProcessedTable {
    let mut processed_data = vec![];
    for row in &table.data {
        let mut out_row = vec![];
        for column in row {
            out_row.push(ProcessedCell {
                contents: split_sublines(&column.contents),
                style: column.style,
            });
        }
        processed_data.push(out_row);
    }

    let mut processed_headers = vec![];
    for header in &table.headers {
        processed_headers.push(ProcessedCell {
            contents: split_sublines(&header.contents),
            style: header.style,
        });
    }

    ProcessedTable {
        headers: processed_headers,
        data: processed_data,
        theme: table.theme.clone(),
    }
}

fn get_max_column_widths(processed_table: &ProcessedTable) -> Vec<usize> {
    use std::cmp::max;

    let mut max_num_columns = 0;

    max_num_columns = max(max_num_columns, processed_table.headers.len());

    for row in &processed_table.data {
        max_num_columns = max(max_num_columns, row.len());
    }

    let mut output = vec![0; max_num_columns];

    for column in processed_table.headers.iter().enumerate() {
        output[column.0] = max(output[column.0], column_width(&column.1.contents));
    }

    for row in &processed_table.data {
        for column in row.iter().enumerate() {
            output[column.0] = max(output[column.0], column_width(&column.1.contents));
        }
    }

    output
}

// pub fn maybe_truncate_columns(
//     termwidth: usize,
//     processed_table: &mut ProcessedTable,
//     max_column_widths: &[usize],
// ) {
//     let (_running_column_width, running_column_count) =
//         get_column_fit_in_termwidth(termwidth, max_column_widths);

//     // Make sure we have enough space for the columns we have
//     let max_num_of_columns = running_column_count - 2;
//     // eprintln!("max_num_of_columns={}", max_num_of_columns);
//     // If we have too many columns, truncate the table
//     if max_num_of_columns < processed_table.headers.len() {
//         processed_table.headers.truncate(max_num_of_columns);

//         for entry in processed_table.data.iter_mut() {
//             entry.truncate(max_num_of_columns);
//         }

//         processed_table.headers.push(ProcessedCell {
//             contents: vec![vec![Subline {
//                 subline: "...",
//                 width: 3,
//             }]],
//             style: TextStyle::basic_center(),
//         });

//         for entry in processed_table.data.iter_mut() {
//             entry.push(ProcessedCell {
//                 contents: vec![vec![Subline {
//                     subline: "...",
//                     width: 3,
//                 }]],
//                 style: TextStyle::basic_center(),
//             }); // ellipsis is centred
//         }
//     }
// }

pub fn draw_table(table: &Table, termwidth: usize, color_hm: &HashMap<String, Style>) {
    // Remove the edges, if used
    // let termwidth = if table.theme.print_left_border && table.theme.print_right_border {
    //     termwidth - 2
    // } else if table.theme.print_left_border || table.theme.print_right_border {
    //     termwidth - 1
    // } else {
    //     termwidth
    // };

    let processed_table = process_table(table);

    // get the max column width for each column using the header and rows
    let max_per_column = get_max_column_widths(&processed_table);

    // maybe_truncate_columns(termwidth, &mut processed_table, &max_per_column);

    // let headers_len = processed_table.headers.len();

    // // fix the length of the table if there are no headers:
    // let headers_len = if headers_len == 0 {
    //     if !table.data.is_empty() && !table.data[0].is_empty() {
    //         table.data[0].len()
    //     } else {
    //         return;
    //     }
    // } else {
    //     headers_len
    // };

    // Measure how big our columns need to be (accounting for separators also)
    // let max_naive_column_width = (termwidth - 2 * (headers_len - 1)) / headers_len;

    // let column_space = ColumnSpace::measure(&max_per_column, max_naive_column_width, headers_len);

    // This gives us the max column width
    // let max_column_width = column_space.max_width(termwidth);

    // This width isn't quite right, as we're rounding off some of our space
    // let column_space = column_space.fix_almost_column_width(
    //     &max_per_column,
    //     max_naive_column_width,
    //     max_column_width,
    //     headers_len,
    // );

    // This should give us the final max column width
    // let max_column_width = column_space.max_width(termwidth);
    let re_leading =
        regex::Regex::new(r"(?P<beginsp>^\s+)").expect("error with leading space regex");
    let re_trailing =
        regex::Regex::new(r"(?P<endsp>\s+$)").expect("error with trailing space regex");

    let (_total_width, total_columns) = get_column_fit_in_termwidth(termwidth, &max_per_column);

    // We really only need this info for the columns we're going to try and fit

    // Get the widest column out of the collection
    let mut modified_max_per_column = max_per_column;
    // eprintln!(
    //     "termwidth=[{}] max_per_col=[{}] total_columns=[{}]",
    //     termwidth,
    //     modified_max_per_column.len(),
    //     total_columns
    // );
    if modified_max_per_column[0] == 1 && modified_max_per_column.len() > total_columns {
        modified_max_per_column = modified_max_per_column[1..=total_columns].to_vec()
    } else {
        modified_max_per_column.truncate(total_columns);
    }
    // eprint!(" mod_max_per_col=[{:?}]", modified_max_per_column);
    let max_column_width = *modified_max_per_column
        .iter()
        .max()
        .expect("failed getting max of columns");
    // eprint!(" max_col_wid=[{}]", max_column_width);
    // Add up the widths of every column
    let all_columns_width_sum: usize = modified_max_per_column.iter().sum();
    // eprint!(" all_col_wid_sum=[{}]", all_columns_width_sum);
    let avg_column_width = all_columns_width_sum / total_columns;
    // eprint!(" avg_col_wid=[{}]", avg_column_width);
    // If the terminal isn't wide enough to fit all columns, modify it
    let mut _modified_max_column_width = 0usize;
    if all_columns_width_sum > termwidth {
        _modified_max_column_width = avg_column_width;
    } else {
        _modified_max_column_width = max_column_width;
    }
    // eprintln!(" mod_max_col_wid=[{}]", _modified_max_column_width);
    // maybe_truncate_columns(termwidth, &mut processed_table, &modified_max_per_column);

    let mut wrapped_table = wrap_cells(
        processed_table,
        _modified_max_column_width,
        total_columns,
        &color_hm,
        &re_leading,
        &re_trailing,
    );
    // eprintln!("mod_max_per_col=[{:?}]", &modified_max_per_column);
    maybe_truncate_wrapped_columns(&mut wrapped_table, termwidth, &modified_max_per_column);

    wrapped_table.print_table(&color_hm);
}

fn get_column_fit_in_termwidth(termwidth: usize, max_column_widths: &[usize]) -> (usize, usize) {
    // How many of the real columns will fit in this max width
    // the number of lines between columns not including the far left and far right
    let column_separators: usize = max_column_widths.len() - 1;
    // start at 2 for far left and far right table line
    let column_left_and_right = 2usize;
    // start point is column separators plus far left and right table lines
    let mut running_column_width = column_separators + column_left_and_right;
    let mut running_column_count = 0usize;
    let mut last_width = 0usize;
    for current_width in max_column_widths.iter() {
        if running_column_width > termwidth {
            running_column_width -= last_width + 2;
            running_column_count -= 1;
            break;
        } else {
            running_column_width += current_width + 2; // each column has at least 1 space on each side
            running_column_count += 1;
            last_width = *current_width;
        }
    }

    (running_column_width, running_column_count)
}

fn maybe_truncate_wrapped_columns(
    wrapped_table: &mut WrappedTable,
    termwidth: usize,
    max_column_widths: &[usize],
) {
    let (_running_column_width, running_column_count) =
        get_column_fit_in_termwidth(termwidth, max_column_widths);

    // Make sure we have enough space for the columns we have
    let max_num_of_columns = running_column_count;

    if max_num_of_columns < wrapped_table.headers.len() {
        // Chop off the headers that are too wide for our terminal
        wrapped_table.headers.truncate(max_num_of_columns);

        // Chop off the data that are too wide for our terminal
        for entry in wrapped_table.data.iter_mut() {
            entry.truncate(max_num_of_columns);
        }

        // Chop off the column_widths too
        wrapped_table.column_widths.truncate(max_num_of_columns);

        // Insert ... for header
        wrapped_table.headers.push(WrappedCell {
            lines: vec![WrappedLine {
                line: "...".to_string(),
                width: 3,
            }],
            max_width: 3,
            style: TextStyle::basic_center(),
        });

        // Insert ... for data
        for entry in wrapped_table.data.iter_mut() {
            entry.push(WrappedCell {
                lines: vec![WrappedLine {
                    line: "...".to_string(),
                    width: 3,
                }],
                max_width: 3,
                style: TextStyle::basic_center(),
            });
        }

        // Insert a column width for ...
        wrapped_table.column_widths.push(3);
    }
}

fn wrap_cells(
    processed_table: ProcessedTable,
    max_column_width: usize,
    total_columns: usize,
    color_hm: &HashMap<String, Style>,
    re_leading: &regex::Regex,
    re_trailing: &regex::Regex,
) -> WrappedTable {
    let mut column_widths = vec![
        0;
        std::cmp::max(
            processed_table.headers.len(),
            if !processed_table.data.is_empty() {
                processed_table.data[0].len()
            } else {
                0
            }
        )
    ];

    // Color for leading and trailing space color from config.toml
    let lead_trail_space_bg_color = color_hm
        .get("leading_trailing_space_bg")
        .unwrap_or(&Style::default())
        .to_owned();

    let mut output_headers = vec![];
    for header in processed_table.headers.into_iter().enumerate() {
        // only process what will show up on screen
        if header.0 > total_columns {
            break;
        }
        let mut wrapped = WrappedCell {
            lines: vec![],
            max_width: 0,
            style: header.1.style,
        };

        for contents in header.1.contents.into_iter() {
            let (mut lines, inner_max_width) = wrap(
                max_column_width,
                contents.into_iter(),
                &re_leading,
                &re_trailing,
                &lead_trail_space_bg_color,
            );
            wrapped.lines.append(&mut lines);
            if inner_max_width > wrapped.max_width {
                wrapped.max_width = inner_max_width;
            }
        }
        if column_widths[header.0] < wrapped.max_width {
            column_widths[header.0] = wrapped.max_width;
        }
        output_headers.push(wrapped);
    }

    let mut output_data = vec![];
    for row in processed_table.data.into_iter() {
        let mut output_row = vec![];
        for column in row.into_iter().enumerate() {
            if column.0 > total_columns {
                break;
            }
            let mut wrapped = WrappedCell {
                lines: vec![],
                max_width: 0,
                style: column.1.style,
            };
            for contents in column.1.contents.into_iter() {
                let (mut lines, inner_max_width) = wrap(
                    max_column_width,
                    contents.into_iter(),
                    &re_leading,
                    &re_trailing,
                    &lead_trail_space_bg_color,
                );
                wrapped.lines.append(&mut lines);
                if inner_max_width > wrapped.max_width {
                    wrapped.max_width = inner_max_width;
                }
            }
            if column_widths[column.0] < wrapped.max_width {
                column_widths[column.0] = wrapped.max_width;
            }
            output_row.push(wrapped);
        }
        output_data.push(output_row);
    }

    WrappedTable {
        column_widths,
        headers: output_headers,
        data: output_data,
        theme: processed_table.theme,
    }
}

// struct ColumnSpace {
//     num_overages: usize,
//     underage_sum: usize,
//     overage_separator_sum: usize,
// }

// impl ColumnSpace {
//     /// Measure how much space we have once we subtract off the columns who are small enough
//     fn measure(
//         max_per_column: &[usize],
//         max_naive_column_width: usize,
//         headers_len: usize,
//     ) -> ColumnSpace {
//         let mut num_overages = 0;
//         let mut underage_sum = 0;
//         let mut overage_separator_sum = 0;
//         let iter = max_per_column.iter().enumerate().take(headers_len);

//         for (i, &column_max) in iter {
//             if column_max > max_naive_column_width {
//                 num_overages += 1;
//                 if i != (headers_len - 1) {
//                     overage_separator_sum += 3;
//                 }
//                 if i == 0 {
//                     overage_separator_sum += 1;
//                 }
//             } else {
//                 underage_sum += column_max;
//                 // if column isn't last, add 3 for its separator
//                 if i != (headers_len - 1) {
//                     underage_sum += 3;
//                 }
//                 if i == 0 {
//                     underage_sum += 1;
//                 }
//             }
//         }

//         ColumnSpace {
//             num_overages,
//             underage_sum,
//             overage_separator_sum,
//         }
//     }

//     fn fix_almost_column_width(
//         self,
//         max_per_column: &[usize],
//         max_naive_column_width: usize,
//         max_column_width: usize,
//         headers_len: usize,
//     ) -> ColumnSpace {
//         let mut num_overages = 0;
//         let mut overage_separator_sum = 0;
//         let mut underage_sum = self.underage_sum;
//         let iter = max_per_column.iter().enumerate().take(headers_len);

//         for (i, &column_max) in iter {
//             if column_max > max_naive_column_width {
//                 if column_max <= max_column_width {
//                     underage_sum += column_max;
//                     // if column isn't last, add 3 for its separator
//                     if i != (headers_len - 1) {
//                         underage_sum += 3;
//                     }
//                     if i == 0 {
//                         underage_sum += 1;
//                     }
//                 } else {
//                     // Column is still too large, so let's count it
//                     num_overages += 1;
//                     if i != (headers_len - 1) {
//                         overage_separator_sum += 3;
//                     }
//                     if i == 0 {
//                         overage_separator_sum += 1;
//                     }
//                 }
//             }
//         }

//         ColumnSpace {
//             num_overages,
//             underage_sum,
//             overage_separator_sum,
//         }
//     }

//     fn max_width(&self, termwidth: usize) -> usize {
//         let ColumnSpace {
//             num_overages,
//             underage_sum,
//             overage_separator_sum,
//         } = self;

//         if *num_overages > 0 {
//             (termwidth - 1 - *underage_sum - *overage_separator_sum) / *num_overages
//         } else {
//             99999
//         }
//     }
// }
