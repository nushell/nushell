use std::collections::HashMap;

use nu_ansi_term::Style;
use nu_protocol::{Config, FooterMode, TrimStrategy};
use tabled::{
    builder::Builder,
    formatting_settings::AlignmentStrategy,
    object::{Cell, Columns, Rows, Segment},
    papergrid,
    style::BorderColor,
    Alignment, AlignmentHorizontal, Modify, ModifyObject, TableOption, Width,
};

use crate::{table_theme::TableTheme, width_control::maybe_truncate_columns, StyledString};

/// Table represent a table view.
#[derive(Debug)]
pub struct Table {
    headers: Option<Vec<StyledString>>,
    data: Vec<Vec<StyledString>>,
    theme: TableTheme,
}

#[derive(Debug)]
pub struct Alignments {
    data: AlignmentHorizontal,
    index: AlignmentHorizontal,
    header: AlignmentHorizontal,
}

impl Default for Alignments {
    fn default() -> Self {
        Self {
            data: AlignmentHorizontal::Center,
            index: AlignmentHorizontal::Right,
            header: AlignmentHorizontal::Center,
        }
    }
}

impl Table {
    /// Creates a [Table] instance.
    ///
    /// If `headers.is_empty` then no headers will be rendered.
    pub fn new(
        headers: Vec<StyledString>,
        data: Vec<Vec<StyledString>>,
        theme: TableTheme,
    ) -> Table {
        let headers = if headers.is_empty() {
            None
        } else {
            Some(headers)
        };

        Table {
            headers,
            data,
            theme,
        }
    }

    /// Draws a trable on a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw_table(
        &self,
        config: &Config,
        color_hm: &HashMap<String, Style>,
        alignments: Alignments,
        termwidth: usize,
    ) -> Option<String> {
        draw_table(self, config, color_hm, alignments, termwidth)
    }
}

fn draw_table(
    table: &Table,
    config: &Config,
    color_hm: &HashMap<String, Style>,
    alignments: Alignments,
    termwidth: usize,
) -> Option<String> {
    let mut headers = colorize_headers(table.headers.as_deref());
    let mut data = colorize_data(&table.data, table.headers.as_ref().map_or(0, |h| h.len()));

    let count_columns = table_fix_lengths(headers.as_mut(), &mut data);

    maybe_truncate_columns(&mut headers, &mut data, count_columns, termwidth);

    let table_data = &table.data;
    let theme = &table.theme;
    let with_header = headers.is_some();
    let with_footer = with_header && need_footer(config, data.len() as u64);
    let with_index = !config.disable_table_indexes;

    let table = build_table(data, headers, with_footer);
    let table = load_theme(table, color_hm, theme, with_footer, with_header);
    let table = align_table(
        table,
        alignments,
        with_index,
        with_header,
        with_footer,
        table_data,
    );
    let table = table_trim_columns(table, termwidth, &config.trim_strategy);

    let table = print_table(table, config);
    if table_width(&table) > termwidth {
        None
    } else {
        Some(table)
    }
}

fn print_table(table: tabled::Table, config: &Config) -> String {
    let output = table.to_string();

    // the atty is for when people do ls from vim, there should be no coloring there
    if !config.use_ansi_coloring || !atty::is(atty::Stream::Stdout) {
        // Draw the table without ansi colors
        match strip_ansi_escapes::strip(&output) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => output, // we did our best; so return at least something
        }
    } else {
        // Draw the table with ansi colors
        output
    }
}

fn table_width(table: &str) -> usize {
    table.lines().next().map_or(0, papergrid::string_width)
}

fn colorize_data(table_data: &[Vec<StyledString>], count_columns: usize) -> Vec<Vec<String>> {
    let mut data = vec![Vec::with_capacity(count_columns); table_data.len()];
    for (row, row_data) in table_data.iter().enumerate() {
        for cell in row_data {
            let colored_text = cell
                .style
                .color_style
                .as_ref()
                .map(|color| color.paint(&cell.contents).to_string())
                .unwrap_or_else(|| cell.contents.clone());

            data[row].push(colored_text)
        }
    }

    data
}

fn colorize_headers(headers: Option<&[StyledString]>) -> Option<Vec<String>> {
    headers.map(|table_headers| {
        let mut headers = Vec::with_capacity(table_headers.len());
        for cell in table_headers {
            let colored_text = cell
                .style
                .color_style
                .as_ref()
                .map(|color| color.paint(&cell.contents).to_string())
                .unwrap_or_else(|| cell.contents.clone());

            headers.push(colored_text)
        }

        headers
    })
}

fn build_table(
    data: Vec<Vec<String>>,
    headers: Option<Vec<String>>,
    need_footer: bool,
) -> tabled::Table {
    let mut builder = Builder::from(data);

    if let Some(headers) = headers {
        builder.set_columns(headers.clone());

        if need_footer {
            builder.add_record(headers);
        }
    }

    builder.build()
}

fn align_table(
    mut table: tabled::Table,
    alignments: Alignments,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
    data: &[Vec<StyledString>],
) -> tabled::Table {
    table = table.with(
        Modify::new(Segment::all())
            .with(Alignment::Horizontal(alignments.data))
            .with(AlignmentStrategy::PerLine),
    );

    if with_index {
        table =
            table.with(Modify::new(Columns::first()).with(Alignment::Horizontal(alignments.index)));
    }

    if with_header {
        let alignment = Alignment::Horizontal(alignments.header);
        table = table.with(Modify::new(Rows::first()).with(alignment.clone()));

        if with_footer {
            table = table.with(Modify::new(Rows::last()).with(alignment));
        }
    }

    table = override_alignments(table, data, with_header, with_index, alignments);

    table
}

fn override_alignments(
    mut table: tabled::Table,
    data: &[Vec<StyledString>],
    header_present: bool,
    index_present: bool,
    alignments: Alignments,
) -> tabled::Table {
    let offset = if header_present { 1 } else { 0 };
    for (row, rows) in data.iter().enumerate() {
        for (col, s) in rows.iter().enumerate() {
            if index_present && col == 0 && s.style.alignment == alignments.index {
                continue;
            }

            if s.style.alignment == alignments.data {
                continue;
            }

            table = table.with(
                Cell(row + offset, col)
                    .modify()
                    .with(Alignment::Horizontal(s.style.alignment)),
            );
        }
    }

    table
}

fn load_theme(
    mut table: tabled::Table,
    color_hm: &HashMap<String, Style>,
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
) -> tabled::Table {
    table = table.with(theme.theme.clone());

    if let Some(color) = color_hm.get("separator") {
        let color = color.paint(" ").to_string();
        if let Ok(color) = BorderColor::try_from(color) {
            table = table.with(color);
        }
    }

    if with_footer {
        table = table.with(FooterStyle).with(
            Modify::new(Rows::last())
                .with(Alignment::center())
                .with(AlignmentStrategy::PerCell),
        );
    }

    if !with_header {
        table = table.with(RemoveHeaderLine);
    }

    table
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}

struct FooterStyle;

impl TableOption for FooterStyle {
    fn change(&mut self, grid: &mut papergrid::Grid) {
        if grid.count_columns() == 0 || grid.count_rows() == 0 {
            return;
        }

        let mut line = papergrid::Line::default();

        let border = grid.get_border((0, 0));
        line.left = border.left_bottom_corner;
        line.intersection = border.right_bottom_corner;
        line.horizontal = border.bottom;

        let border = grid.get_border((0, grid.count_columns() - 1));
        line.right = border.right_bottom_corner;

        grid.set_split_line(grid.count_rows() - 1, line);
    }
}

struct RemoveHeaderLine;

impl TableOption for RemoveHeaderLine {
    fn change(&mut self, grid: &mut papergrid::Grid) {
        grid.set_split_line(1, papergrid::Line::default());
    }
}

struct CountColumns(usize);

impl TableOption for &mut CountColumns {
    fn change(&mut self, grid: &mut papergrid::Grid) {
        self.0 = grid.count_columns();
    }
}

fn table_trim_columns(
    table: tabled::Table,
    termwidth: usize,
    trim_strategy: &TrimStrategy,
) -> tabled::Table {
    table.with(&TrimStrategyModifier {
        termwidth,
        trim_strategy,
    })
}

pub struct TrimStrategyModifier<'a> {
    termwidth: usize,
    trim_strategy: &'a TrimStrategy,
}

impl tabled::TableOption for &TrimStrategyModifier<'_> {
    fn change(&mut self, grid: &mut papergrid::Grid) {
        match self.trim_strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let mut w = Width::wrap(self.termwidth).priority::<tabled::width::PriorityMax>();
                if *try_to_keep_words {
                    w = w.keep_words();
                }

                w.change(grid)
            }
            TrimStrategy::Truncate { suffix } => {
                let mut w =
                    Width::truncate(self.termwidth).priority::<tabled::width::PriorityMax>();
                if let Some(suffix) = suffix {
                    w = w.suffix(suffix).suffix_try_color(true);
                }

                w.change(grid);
            }
        };
    }
}

fn table_fix_lengths(headers: Option<&mut Vec<String>>, data: &mut [Vec<String>]) -> usize {
    let length = table_find_max_length(headers.as_deref(), data);

    if let Some(headers) = headers {
        headers.extend(std::iter::repeat(String::default()).take(length - headers.len()));
    }

    for row in data {
        row.extend(std::iter::repeat(String::default()).take(length - row.len()));
    }

    length
}

fn table_find_max_length<T>(headers: Option<&Vec<T>>, data: &[Vec<T>]) -> usize {
    let mut length = headers.map_or(0, |h| h.len());
    for row in data {
        length = std::cmp::max(length, row.len());
    }

    length
}
