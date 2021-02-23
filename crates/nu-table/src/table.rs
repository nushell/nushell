use crate::wrap::{column_width, split_sublines, wrap, Alignment, Subline, WrappedCell};
use nu_ansi_term::{Color, Style};
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

    pub fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub alignment: Alignment,
    pub color_style: Option<Style>,
}

impl TextStyle {
    pub fn new() -> TextStyle {
        TextStyle {
            alignment: Alignment::Left,
            color_style: Some(Style::default()),
        }
    }

    pub fn bold(&self, bool_value: Option<bool>) -> TextStyle {
        let bv = bool_value.unwrap_or(false);

        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_bold: bv,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_bold(&self) -> bool {
        self.color_style.unwrap_or_default().is_bold
    }

    pub fn dimmed(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_dimmed: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_dimmed(&self) -> bool {
        self.color_style.unwrap_or_default().is_dimmed
    }

    pub fn italic(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_italic: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_italic(&self) -> bool {
        self.color_style.unwrap_or_default().is_italic
    }

    pub fn underline(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_underline: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_underline(&self) -> bool {
        self.color_style.unwrap_or_default().is_underline
    }

    pub fn blink(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_blink: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_blink(&self) -> bool {
        self.color_style.unwrap_or_default().is_blink
    }

    pub fn reverse(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_reverse: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_reverse(&self) -> bool {
        self.color_style.unwrap_or_default().is_reverse
    }

    pub fn hidden(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_hidden: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.color_style.unwrap_or_default().is_hidden
    }

    pub fn strikethrough(&self) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                is_strikethrough: true,
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn is_strikethrough(&self) -> bool {
        self.color_style.unwrap_or_default().is_strikethrough
    }

    pub fn fg(&self, foreground: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                foreground: Some(foreground),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn on(&self, background: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                background: Some(background),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn bg(&self, background: Color) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                background: Some(background),
                ..self.color_style.unwrap_or_default()
            }),
        }
    }

    pub fn alignment(&self, align: Alignment) -> TextStyle {
        TextStyle {
            alignment: align,
            color_style: self.color_style,
        }
    }

    pub fn style(&self, style: Style) -> TextStyle {
        TextStyle {
            alignment: self.alignment,
            color_style: Some(Style {
                foreground: style.foreground,
                background: style.background,
                is_bold: style.is_bold,
                is_dimmed: style.is_dimmed,
                is_italic: style.is_italic,
                is_underline: style.is_underline,
                is_blink: style.is_blink,
                is_reverse: style.is_reverse,
                is_hidden: style.is_hidden,
                is_strikethrough: style.is_strikethrough,
            }),
        }
    }

    pub fn basic_center() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Center)
            .style(Style::default())
    }

    pub fn basic_right() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Right)
            .style(Style::default())
    }

    pub fn basic_left() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Left)
            .style(Style::default())
    }

    pub fn default_header() -> TextStyle {
        TextStyle::new()
            .alignment(Alignment::Center)
            .fg(Color::Green)
            .bold(Some(true))
    }

    pub fn with_attributes(bo: bool, al: Alignment, co: Color) -> TextStyle {
        TextStyle::new().alignment(al).fg(co).bold(Some(bo))
    }

    pub fn with_style(al: Alignment, style: Style) -> TextStyle {
        TextStyle::new().alignment(al).style(Style {
            foreground: style.foreground,
            background: style.background,
            is_bold: style.is_bold,
            is_dimmed: style.is_dimmed,
            is_italic: style.is_italic,
            is_underline: style.is_underline,
            is_blink: style.is_blink,
            is_reverse: style.is_reverse,
            is_hidden: style.is_hidden,
            is_strikethrough: style.is_strikethrough,
        })
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
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

    #[allow(unused)]
    pub fn with_love() -> Theme {
        Theme {
            top_left: '❤',
            middle_left: '❤',
            bottom_left: '❤',
            top_center: '❤',
            center: '❤',
            bottom_center: '❤',
            top_right: '❤',
            middle_right: '❤',
            bottom_right: '❤',
            top_horizontal: '❤',
            middle_horizontal: '❤',
            bottom_horizontal: '❤',

            left_vertical: ' ',
            center_vertical: '❤',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn compact_double() -> Theme {
        Theme {
            top_left: '═',
            middle_left: '═',
            bottom_left: '═',
            top_center: '╦',
            center: '╬',
            bottom_center: '╩',
            top_right: '═',
            middle_right: '═',
            bottom_right: '═',
            top_horizontal: '═',
            middle_horizontal: '═',
            bottom_horizontal: '═',

            left_vertical: ' ',
            center_vertical: '║',
            right_vertical: ' ',

            separate_header: true,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn rounded() -> Theme {
        Theme {
            top_left: '╭',
            middle_left: '├',
            bottom_left: '╰',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '╮',
            middle_right: '┤',
            bottom_right: '╯',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn reinforced() -> Theme {
        Theme {
            top_left: '┏',
            middle_left: '├',
            bottom_left: '┗',
            top_center: '┬',
            center: '┼',
            bottom_center: '┴',
            top_right: '┓',
            middle_right: '┤',
            bottom_right: '┛',
            top_horizontal: '─',
            middle_horizontal: '─',
            bottom_horizontal: '─',

            left_vertical: '│',
            center_vertical: '│',
            right_vertical: '│',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }

    #[allow(unused)]
    pub fn heavy() -> Theme {
        Theme {
            top_left: '┏',
            middle_left: '┣',
            bottom_left: '┗',
            top_center: '┳',
            center: '╋',
            bottom_center: '┻',
            top_right: '┓',
            middle_right: '┫',
            bottom_right: '┛',
            top_horizontal: '━',
            middle_horizontal: '━',
            bottom_horizontal: '━',

            left_vertical: '┃',
            center_vertical: '┃',
            right_vertical: '┃',

            separate_header: true,
            separate_rows: false,

            print_left_border: true,
            print_right_border: true,
            print_top_border: true,
            print_bottom_border: true,
        }
    }
    #[allow(unused)]
    pub fn none() -> Theme {
        Theme {
            top_left: ' ',
            middle_left: ' ',
            bottom_left: ' ',
            top_center: ' ',
            center: ' ',
            bottom_center: ' ',
            top_right: ' ',
            middle_right: ' ',
            bottom_right: ' ',

            top_horizontal: ' ',
            middle_horizontal: ' ',
            bottom_horizontal: ' ',

            left_vertical: ' ',
            center_vertical: ' ',
            right_vertical: ' ',

            separate_header: false,
            separate_rows: false,

            print_left_border: false,
            print_right_border: false,
            print_top_border: false,
            print_bottom_border: false,
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
    fn print_separator(
        &self,
        separator_position: SeparatorPosition,
        color_hm: &HashMap<String, Style>,
    ) -> String {
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
        output.push('\n');
        output
    }

    fn print_cell_contents(
        &self,
        cells: &[WrappedCell],
        color_hm: &HashMap<String, Style>,
    ) -> String {
        let sep_color = color_hm
            .get("separator_color")
            .unwrap_or(&Style::default())
            .to_owned();

        let mut total_output = String::new();

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
                total_output.push_str(output.as_str());
                total_output.push('\n');
            }
        }
        total_output
    }

    fn print_table(&self, color_hm: &HashMap<String, Style>) -> String {
        let mut output = String::new();

        #[cfg(windows)]
        {
            let _ = nu_ansi_term::enable_ansi_support();
        }

        if self.data.is_empty() {
            return output;
        }

        if self.theme.print_top_border {
            output.push_str(
                self.print_separator(SeparatorPosition::Top, &color_hm)
                    .as_str(),
            );
        }

        let skip_headers = (self.headers.len() == 2 && self.headers[1].max_width == 0)
            || (self.headers.len() == 1 && self.headers[0].max_width == 0);

        if !self.headers.is_empty() && !skip_headers {
            output.push_str(self.print_cell_contents(&self.headers, &color_hm).as_str());
        }

        let mut first_row = true;

        for row in &self.data {
            if !first_row {
                if self.theme.separate_rows {
                    output.push_str(
                        self.print_separator(SeparatorPosition::Middle, &color_hm)
                            .as_str(),
                    )
                }
            } else {
                first_row = false;

                if self.theme.separate_header && !self.headers.is_empty() && !skip_headers {
                    output.push_str(
                        self.print_separator(SeparatorPosition::Middle, &color_hm)
                            .as_str(),
                    );
                }
            }

            output.push_str(self.print_cell_contents(row, &color_hm).as_str());
        }

        if self.theme.print_bottom_border {
            output.push_str(
                self.print_separator(SeparatorPosition::Bottom, &color_hm)
                    .as_str(),
            );
        }
        output
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

pub fn maybe_truncate_columns(termwidth: usize, processed_table: &mut ProcessedTable) {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;

    // If we have too many columns, truncate the table
    if max_num_of_columns < processed_table.headers.len() {
        processed_table.headers.truncate(max_num_of_columns);

        for entry in processed_table.data.iter_mut() {
            entry.truncate(max_num_of_columns);
        }

        processed_table.headers.push(ProcessedCell {
            contents: vec![vec![Subline {
                subline: "...",
                width: 3,
            }]],
            style: TextStyle::basic_center(),
        });

        for entry in processed_table.data.iter_mut() {
            entry.push(ProcessedCell {
                contents: vec![vec![Subline {
                    subline: "...",
                    width: 3,
                }]],
                style: TextStyle::basic_center(),
            }); // ellipsis is centred
        }
    }
}

pub fn draw_table(table: &Table, termwidth: usize, color_hm: &HashMap<String, Style>) -> String {
    // Remove the edges, if used
    let termwidth = if table.theme.print_left_border && table.theme.print_right_border {
        termwidth - 2
    } else if table.theme.print_left_border || table.theme.print_right_border {
        termwidth - 1
    } else {
        termwidth
    };

    let mut processed_table = process_table(table);

    let max_per_column = get_max_column_widths(&processed_table);

    maybe_truncate_columns(termwidth, &mut processed_table);

    let headers_len = processed_table.headers.len();

    // fix the length of the table if there are no headers:
    let headers_len = if headers_len == 0 {
        if !table.data.is_empty() && !table.data[0].is_empty() {
            table.data[0].len()
        } else {
            return String::new();
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
    let re_leading =
        regex::Regex::new(r"(?P<beginsp>^\s+)").expect("error with leading space regex");
    let re_trailing =
        regex::Regex::new(r"(?P<endsp>\s+$)").expect("error with trailing space regex");

    let wrapped_table = wrap_cells(
        processed_table,
        max_column_width,
        &color_hm,
        &re_leading,
        &re_trailing,
    );

    wrapped_table.print_table(&color_hm)
}

fn wrap_cells(
    processed_table: ProcessedTable,
    max_column_width: usize,
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
    let mut output_headers = vec![];
    for header in processed_table.headers.into_iter().enumerate() {
        let mut wrapped = WrappedCell {
            lines: vec![],
            max_width: 0,
            style: header.1.style,
        };

        for contents in header.1.contents.into_iter() {
            let (mut lines, inner_max_width) = wrap(
                max_column_width,
                contents.into_iter(),
                &color_hm,
                &re_leading,
                &re_trailing,
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
            let mut wrapped = WrappedCell {
                lines: vec![],
                max_width: 0,
                style: column.1.style,
            };
            for contents in column.1.contents.into_iter() {
                let (mut lines, inner_max_width) = wrap(
                    max_column_width,
                    contents.into_iter(),
                    &color_hm,
                    &re_leading,
                    &re_trailing,
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
