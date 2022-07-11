use std::collections::HashMap;

use nu_ansi_term::Style;
use nu_protocol::{Config, FooterMode, TrimStrategy};
use tabled::{
    builder::Builder,
    formatting_settings::AlignmentStrategy,
    object::{Cell, Columns, Rows},
    papergrid,
    style::BorderColor,
    Alignment, Modify, TableOption, Width,
};

use crate::{
    table_theme::TableTheme,
    width_control::{estimate_max_column_width, fix_termwidth, maybe_truncate_columns},
    StyledString,
};

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

pub fn draw_table(
    table: &Table,
    termwidth: usize,
    color_hm: &HashMap<String, Style>,
    config: &Config,
) -> Option<String> {
    let termwidth = fix_termwidth(termwidth, &table.theme)?;

    let (mut headers, mut data) = table_fix_lengths(&table.headers, &table.data);

    maybe_truncate_columns(&mut headers, &mut data, termwidth);

    let max_column_width = estimate_max_column_width(&headers, &data, termwidth)?;

    let alignments = build_alignment_map(&table.data);

    let headers = table_header_to_strings(headers);
    let data = table_data_to_strings(data, headers.len());

    let headers = if headers.is_empty() {
        None
    } else {
        Some(headers)
    };

    let theme = &table.theme;
    let with_header = headers.is_some();
    let with_footer = with_header && need_footer(config, data.len() as u64);

    let table = build_table(data, headers, Some(alignments), config, with_footer);
    let table = load_theme(table, color_hm, theme, with_footer, with_header);

    let (count_columns, table) = count_columns_on_table(table);

    let table = table_trim_columns(
        table,
        count_columns,
        termwidth,
        max_column_width,
        &config.trim_strategy,
    );

    Some(table.to_string())
}

fn count_columns_on_table(mut table: tabled::Table) -> (usize, tabled::Table) {
    let mut c = CountColumns(0);
    table = table.with(&mut c);

    (c.0, table)
}

fn table_data_to_strings(
    table_data: Vec<Vec<StyledString>>,
    count_headers: usize,
) -> Vec<Vec<String>> {
    let mut data = vec![Vec::with_capacity(count_headers); table_data.len()];
    for (row, row_data) in table_data.into_iter().enumerate() {
        for cell in row_data {
            let colored_text = cell
                .style
                .color_style
                .as_ref()
                .map(|color| color.paint(&cell.contents).to_string())
                .unwrap_or(cell.contents);

            data[row].push(colored_text)
        }
    }

    data
}

fn table_header_to_strings(table_headers: Vec<StyledString>) -> Vec<String> {
    let mut headers = Vec::with_capacity(table_headers.len());
    for cell in table_headers {
        let colored_text = cell
            .style
            .color_style
            .as_ref()
            .map(|color| color.paint(&cell.contents).to_string())
            .unwrap_or(cell.contents);

        headers.push(colored_text)
    }

    headers
}

fn build_alignment_map(data: &[Vec<StyledString>]) -> Vec<Vec<Alignment>> {
    let mut v = vec![Vec::new(); data.len()];
    for (i, row) in data.iter().enumerate() {
        let mut row_alignments = Vec::with_capacity(row.len());
        for col in row {
            row_alignments.push(Alignment::Horizontal(col.style.alignment));
        }

        v[i] = row_alignments;
    }

    v
}

fn build_table(
    data: Vec<Vec<String>>,
    headers: Option<Vec<String>>,
    alignment_map: Option<Vec<Vec<Alignment>>>,
    config: &Config,
    need_footer: bool,
) -> tabled::Table {
    let header_present = headers.is_some();
    let mut builder = Builder::from(data);

    if let Some(headers) = headers {
        builder = builder.set_columns(headers.clone());

        if need_footer {
            builder = builder.add_record(headers);
        }
    }

    let mut table = builder.build();

    table = table.with(
        Modify::new(Rows::new(1..))
            .with(Alignment::left())
            .with(AlignmentStrategy::PerLine),
    );

    if !config.disable_table_indexes {
        table = table.with(Modify::new(Columns::first()).with(Alignment::right()));
    }

    if header_present {
        table = table.with(Modify::new(Rows::first()).with(Alignment::center()));
    }

    if let Some(alignments) = alignment_map {
        table = apply_alignments(table, alignments, header_present);
    }

    table
}

fn apply_alignments(
    mut table: tabled::Table,
    alignment: Vec<Vec<Alignment>>,
    header_present: bool,
) -> tabled::Table {
    let offset = if header_present { 1 } else { 0 };
    for (row, alignments) in alignment.into_iter().enumerate() {
        for (col, alignment) in alignments.into_iter().enumerate() {
            table = table.with(Modify::new(Cell(row + offset, col)).with(alignment));
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
    count_columns: usize,
    termwidth: usize,
    max_column_width: usize,
    trim_strategy: &TrimStrategy,
) -> tabled::Table {
    let mut table_width = max_column_width * count_columns;
    if table_width > termwidth {
        table_width = termwidth;
    }

    table.with(&TrimStrategyModifier {
        termwidth: table_width,
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
                let mut w = Width::wrap(self.termwidth);
                if *try_to_keep_words {
                    w = w.keep_words();
                }
                let mut w = w.priority::<tabled::width::PriorityMax>();

                w.change(grid)
            }
            TrimStrategy::Truncate { suffix } => {
                let mut w =
                    Width::truncate(self.termwidth).priority::<tabled::width::PriorityMax>();
                if let Some(suffix) = suffix {
                    w = w.suffix(suffix);
                }

                w.change(grid);
            }
        };
    }
}

fn table_fix_lengths(
    headers: &[StyledString],
    data: &[Vec<StyledString>],
) -> (Vec<StyledString>, Vec<Vec<StyledString>>) {
    let length = table_find_max_length(headers, data);

    let mut headers_fixed = Vec::with_capacity(length);
    headers_fixed.extend(headers.iter().cloned());
    headers_fixed.extend(std::iter::repeat(StyledString::default()).take(length - headers.len()));

    let mut data_fixed = Vec::with_capacity(data.len());
    for row in data {
        let mut row_fixed = Vec::with_capacity(length);
        row_fixed.extend(row.iter().cloned());
        row_fixed.extend(std::iter::repeat(StyledString::default()).take(length - row.len()));
        data_fixed.push(row_fixed);
    }

    (headers_fixed, data_fixed)
}

fn table_find_max_length(headers: &[StyledString], data: &[Vec<StyledString>]) -> usize {
    let mut length = headers.len();
    for row in data {
        length = std::cmp::max(length, row.len());
    }

    length
}
