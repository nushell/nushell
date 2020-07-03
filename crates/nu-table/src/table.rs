use crate::wrap::{column_width, split_sublines, wrap, Alignment, Subline, WrappedCell};

enum SeparatorPosition {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug)]
pub struct Table {
    pub headers: Vec<StyledString>,
    pub data: Vec<Vec<StyledString>>,
    pub theme: Theme,
}

#[derive(Debug, Clone)]
pub struct StyledString {
    pub contents: String,
    pub style: TextStyle,
}

impl StyledString {
    pub fn new(contents: String, style: TextStyle) -> StyledString {
        StyledString { contents, style }
    }
}

#[derive(Debug, Clone)]
pub struct TextStyle {
    pub is_bold: bool,
    pub alignment: Alignment,
    pub color: Option<ansi_term::Colour>,
}

impl TextStyle {
    pub fn basic() -> TextStyle {
        TextStyle {
            is_bold: false,
            alignment: Alignment::Left,
            color: None,
        }
    }

    pub fn basic_right() -> TextStyle {
        TextStyle {
            is_bold: false,
            alignment: Alignment::Right,
            color: None,
        }
    }

    pub fn default_header() -> TextStyle {
        TextStyle {
            is_bold: true,
            alignment: Alignment::Center,
            color: Some(ansi_term::Colour::Green),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub top_left: char,
    pub middle_left: char,
    pub bottom_left: char,
    pub top_center: char,
    pub center: char,
    pub bottom_center: char,
    pub top_right: char,
    pub middle_right: char,
    pub bottom_right: char,
    pub top_horizontal: char,
    pub middle_horizontal: char,
    pub bottom_horizontal: char,
    pub left_vertical: char,
    pub center_vertical: char,
    pub right_vertical: char,

    pub separate_header: bool,
    pub separate_rows: bool,

    pub print_left_border: bool,
    pub print_right_border: bool,
    pub print_top_border: bool,
    pub print_bottom_border: bool,
}

impl Theme {
    #[allow(unused)]
    pub fn basic() -> Theme {
        Theme {
            top_left: '+',
            middle_left: '+',
            bottom_left: '+',
            top_center: '+',
            center: '+',
            bottom_center: '+',
            top_right: '+',
            middle_right: '+',
            bottom_right: '+',
            top_horizontal: '-',
            middle_horizontal: '-',
            bottom_horizontal: '-',
            left_vertical: '|',
            center_vertical: '|',
            right_vertical: '|',

            separate_header: true,
            separate_rows: true,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }
    #[allow(unused)]
    pub fn thin() -> Theme {
        Theme {
            top_left: '┌',
            middle_left: '├',
            bottom_left: '└',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '┐',
            middle_right: '┤',
            bottom_right: '┘',

            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: true,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }
    #[allow(unused)]
    pub fn light() -> Theme {
        Theme {
            top_left: ' ',
            middle_left: '─',
            bottom_left: ' ',
            top_center: ' ',
            center: '─',
            bottom_center: ' ',
            top_right: ' ',
            middle_right: '─',
            bottom_right: ' ',

            top_horizontal: ' ',
            middle_horizontal: '─',
            bottom_horizontal: ' ',

            left_vertical: ' ',
            center_vertical: ' ',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: false,
            print_bottom_border: true,
        }
    }
    #[allow(unused)]
    pub fn compact() -> Theme {
        Theme {
            top_left: '─',
            middle_left: '─',
            bottom_left: '─',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '─',
            middle_right: '─',
            bottom_right: '─',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: ' ',
            center_vertical: '│',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }
}

impl Table {
    pub fn new(headers: Vec<StyledString>, data: Vec<Vec<StyledString>>, theme: Theme) -> Table {
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
    pub theme: Theme,
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
    pub theme: Theme,
}

impl WrappedTable {
    fn print_separator(&self, separator_position: SeparatorPosition) {
        let column_count = self.column_widths.len();
        let mut output = String::new();

        match separator_position {
            SeparatorPosition::Top => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push(self.theme.top_left);
                    }

                    for _ in 0..*column.1 {
                        output.push(self.theme.top_horizontal);
                    }

                    output.push(self.theme.top_horizontal);
                    output.push(self.theme.top_horizontal);
                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push(self.theme.top_right);
                        }
                    } else {
                        output.push(self.theme.top_center);
                    }
                }
            }
            SeparatorPosition::Middle => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push(self.theme.middle_left);
                    }

                    for _ in 0..*column.1 {
                        output.push(self.theme.middle_horizontal);
                    }

                    output.push(self.theme.middle_horizontal);
                    output.push(self.theme.middle_horizontal);

                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push(self.theme.middle_right);
                        }
                    } else {
                        output.push(self.theme.center);
                    }
                }
            }
            SeparatorPosition::Bottom => {
                for column in self.column_widths.iter().enumerate() {
                    if column.0 == 0 && self.theme.print_left_border {
                        output.push(self.theme.bottom_left);
                    }
                    for _ in 0..*column.1 {
                        output.push(self.theme.bottom_horizontal);
                    }
                    output.push(self.theme.bottom_horizontal);
                    output.push(self.theme.bottom_horizontal);

                    if column.0 == column_count - 1 {
                        if self.theme.print_right_border {
                            output.push(self.theme.bottom_right);
                        }
                    } else {
                        output.push(self.theme.bottom_center);
                    }
                }
            }
        }

        println!("{}", output);
    }

    fn print_cell_contents(&self, cells: &[WrappedCell]) {
        for current_line in 0.. {
            let mut lines_printed = 0;

            let mut output = if self.theme.print_left_border {
                self.theme.left_vertical.to_string()
            } else {
                String::new()
            };
            for column in cells.iter().enumerate() {
                if let Some(line) = (column.1).lines.get(current_line) {
                    let remainder = self.column_widths[column.0] - line.width;
                    output.push(' ');

                    match column.1.style.alignment {
                        Alignment::Left => {
                            if let Some(color) = column.1.style.color {
                                let color = if column.1.style.is_bold {
                                    color.bold()
                                } else {
                                    color.normal()
                                };

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
                            if let Some(color) = column.1.style.color {
                                let color = if column.1.style.is_bold {
                                    color.bold()
                                } else {
                                    color.normal()
                                };

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
                            if let Some(color) = column.1.style.color {
                                let color = if column.1.style.is_bold {
                                    color.bold()
                                } else {
                                    color.normal()
                                };

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
                    output.push(self.theme.center_vertical);
                } else if self.theme.print_right_border {
                    output.push(self.theme.right_vertical);
                }
            }
            if lines_printed == 0 {
                break;
            } else {
                println!("{}", output);
            }
        }
    }
    fn new_print_table(&self) {
        if self.data.is_empty() {
            return;
        }

        if self.theme.print_top_border {
            self.print_separator(SeparatorPosition::Top);
        }

        let skip_headers = (self.headers.len() == 2 && self.headers[1].max_width == 0)
            || (self.headers.len() == 1 && self.headers[0].max_width == 0);

        if !self.headers.is_empty() && !skip_headers {
            self.print_cell_contents(&self.headers);
        }

        let mut first_row = true;

        for row in &self.data {
            if !first_row {
                if self.theme.separate_rows {
                    self.print_separator(SeparatorPosition::Middle);
                }
            } else {
                first_row = false;

                if self.theme.separate_header && !self.headers.is_empty() && !skip_headers {
                    self.print_separator(SeparatorPosition::Middle);
                }
            }

            self.print_cell_contents(row);
        }

        if self.theme.print_bottom_border {
            self.print_separator(SeparatorPosition::Bottom);
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
                style: column.style.clone(),
            });
        }
        processed_data.push(out_row);
    }

    let mut processed_headers = vec![];
    for header in &table.headers {
        processed_headers.push(ProcessedCell {
            contents: split_sublines(&header.contents),
            style: header.style.clone(),
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

pub fn draw_table(table: &Table, termwidth: usize) {
    // Remove the edges, if used
    let termwidth = if table.theme.print_left_border && table.theme.print_right_border {
        termwidth - 2
    } else if table.theme.print_left_border || table.theme.print_right_border {
        termwidth - 1
    } else {
        termwidth
    };

    let processed_table = process_table(table);

    let max_per_column = get_max_column_widths(&processed_table);

    // maybe_truncate_columns(&mut headers, &mut entries, termwidth);
    let headers_len = table.headers.len();

    // fix the length of the table if there are no headers:
    let headers_len = if headers_len == 0 {
        if !table.data.is_empty() && !table.data[0].is_empty() {
            table.data[0].len()
        } else {
            return;
        }
    } else {
        headers_len
    };

    // Measure how big our columns need to be (accounting for separators also)
    let max_naive_column_width = (termwidth - 3 * (headers_len - 1)) / headers_len;

    let column_space = ColumnSpace::measure(&max_per_column, max_naive_column_width, headers_len);

    // This gives us the max column width
    let max_column_width = column_space.max_width(termwidth);

    // This width isn't quite right, as we're rounding off some of our space
    let column_space = column_space.fix_almost_column_width(
        &max_per_column,
        max_naive_column_width,
        max_column_width,
        headers_len,
    );

    // This should give us the final max column width
    let max_column_width = column_space.max_width(termwidth);

    let wrapped_table = wrap_cells(processed_table, max_column_width);

    wrapped_table.new_print_table();
}

fn wrap_cells(processed_table: ProcessedTable, max_column_width: usize) -> WrappedTable {
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
    let mut output_headers = vec![];
    for header in processed_table.headers.into_iter().enumerate() {
        let mut wrapped = WrappedCell {
            lines: vec![],
            max_width: 0,
            style: header.1.style,
        };

        for contents in header.1.contents.into_iter() {
            let (mut lines, inner_max_width) = wrap(max_column_width, contents.into_iter());
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
            let mut wrapped = WrappedCell {
                lines: vec![],
                max_width: 0,
                style: column.1.style,
            };
            for contents in column.1.contents.into_iter() {
                let (mut lines, inner_max_width) = wrap(max_column_width, contents.into_iter());
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

    fn max_width(&self, termwidth: usize) -> usize {
        let ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        } = self;

        if *num_overages > 0 {
            (termwidth - 1 - *underage_sum - *overage_separator_sum) / *num_overages
        } else {
            99999
        }
    }
}
