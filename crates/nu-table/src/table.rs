use crate::table_theme::TableTheme;
use crate::StyledString;
use nu_ansi_term::Style;
use nu_protocol::{Config, FooterMode};
use std::collections::HashMap;
use tabled::formatting_settings::AlignmentStrategy;
use tabled::object::{Cell, Columns, Rows};
use tabled::style::StyleConfig;
use tabled::{Alignment, Modify};

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
    // Remove the edges, if used
    let (headers, data) = crate::wrap::wrap(&table.headers, &table.data, termwidth, &table.theme)?;
    let headers = if headers.is_empty() {
        None
    } else {
        Some(headers)
    };

    let alignments = build_alignment_map(&table.data);

    let theme = &table.theme;

    let with_header = headers.is_some();
    let with_footer = with_header && need_footer(config, data.len() as u64);

    let table = build_table(data, headers, Some(alignments), config, with_footer);
    let table = load_theme(table, color_hm, theme, with_footer, with_header);

    print_table(table, termwidth)
}

fn print_table(table: tabled::Table, term_width: usize) -> Option<String> {
    let s = table.to_string();

    let width = s
        .lines()
        .next()
        .map(tabled::papergrid::string_width)
        .unwrap_or(0);
    if width > term_width {
        return None;
    }

    Some(s)
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
    let mut builder = tabled::builder::Builder::from(data);

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

fn nu_theme_to_tabled(theme: &TableTheme) -> StyleConfig {
    let mut t: StyleConfig = tabled::style::Style::blank()
        .top(theme.top_horizontal)
        .bottom(theme.bottom_horizontal)
        .left(theme.left_vertical)
        .right(theme.right_vertical)
        .horizontal(theme.middle_horizontal)
        .header(theme.middle_horizontal)
        .vertical(theme.center_vertical)
        .inner_intersection(theme.center)
        .header_intersection(theme.center)
        .top_intersection(theme.top_center)
        .top_left_corner(theme.top_left)
        .top_right_corner(theme.top_right)
        .bottom_intersection(theme.bottom_center)
        .bottom_left_corner(theme.bottom_left)
        .bottom_right_corner(theme.bottom_right)
        .left_intersection(theme.middle_left)
        .right_intersection(theme.middle_right)
        .into();

    if !theme.separate_rows {
        t.set_horizontal(None);
    }

    if !theme.separate_header {
        t.set_header(None);
    }

    if !theme.print_top_border {
        t.set_top(None);
    }

    if !theme.print_bottom_border {
        t.set_bottom(None);
    }

    if !theme.print_left_border {
        t.set_left(None);

        if !theme.print_top_border {
            t.set_top_left(None);
        }

        if !theme.print_bottom_border {
            t.set_bottom_left(None);
        }
    }

    if !theme.print_right_border {
        t.set_right(None);

        if !theme.print_top_border {
            t.set_top_right(None);
        }

        if !theme.print_bottom_border {
            t.set_bottom_right(None);
        }
    }

    t
}

fn load_theme(
    mut table: tabled::Table,
    color_hm: &HashMap<String, Style>,
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
) -> tabled::Table {
    let style = nu_theme_to_tabled(theme);
    table = table.with(style);

    if let Some(color) = color_hm.get("separator") {
        let color = color.paint(" ").to_string();
        table = table.with(tabled::style::BorderColor::try_from(color).unwrap());
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

impl tabled::TableOption for FooterStyle {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        if grid.count_columns() == 0 || grid.count_rows() == 0 {
            return;
        }

        let mut line = tabled::papergrid::Line::default();

        let border = grid.get_border((0, 0));
        line.left = border.left_bottom_corner;
        line.intersection = border.right_bottom_corner;
        line.horizontal = border.bottom;

        let border = grid.get_border((0, grid.count_columns() - 1));
        line.right = border.right_bottom_corner;

        grid.set_split_line(grid.count_rows() - 1, line);
    }
}

struct CalculateTableWidth(usize);

impl tabled::TableOption for CalculateTableWidth {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        self.0 = grid.total_width();
    }
}

struct RemoveHeaderLine;

impl tabled::TableOption for RemoveHeaderLine {
    fn change(&mut self, grid: &mut tabled::papergrid::Grid) {
        grid.set_split_line(1, tabled::papergrid::Line::default());
    }
}
