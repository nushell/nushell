use std::{collections::HashMap, fmt::Display, iter};

use nu_protocol::{Config, FooterMode, TrimStrategy};
use tabled::{
    alignment::AlignmentHorizontal,
    builder::Builder,
    color::Color,
    formatting::AlignmentStrategy,
    object::{Cell, Columns, Rows, Segment},
    papergrid::{
        self,
        records::{self, records_info_colored::RecordsInfo, Records, RecordsMut},
        GridConfig,
    },
    Alignment, Modify, ModifyObject, TableOption, Width,
};

use crate::{
    table_theme::TableTheme, width_control::maybe_truncate_columns, StyledString, TextStyle,
};

/// Table represent a table view.
#[derive(Debug)]
pub struct Table<'a> {
    data: RecordsInfo<'a, TextStyle>,
    with_header: bool,
    is_empty: bool,
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

impl<'a> Table<'a> {
    /// Creates a [Table] instance.
    ///
    /// If `headers.is_empty` then no headers will be rendered.
    pub fn new<D, DR>(
        data: D,
        size: (usize, usize),
        termwidth: usize,
        with_header: bool,
    ) -> Table<'a>
    where
        D: IntoIterator<Item = DR> + 'a,
        DR: IntoIterator<Item = StyledString> + 'a,
    {
        let data = data
            .into_iter()
            .map(|row| row.into_iter().map(|c| (c.contents, c.style)));

        let mut data = RecordsInfo::new(data, size, &GridConfig::default());

        let count_columns = (&data).size().1;
        let is_empty = maybe_truncate_columns(&mut data, count_columns, termwidth);

        Table {
            data,
            is_empty,
            with_header,
        }
    }

    /// Draws a trable on a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw_table(
        &self,
        config: &Config,
        color_hm: &HashMap<String, nu_ansi_term::Style>,
        alignments: Alignments,
        theme: &TableTheme,
        termwidth: usize,
    ) -> Option<String> {
        draw_table(self, config, color_hm, alignments, theme, termwidth)
    }
}

fn draw_table(
    table: &Table,
    config: &Config,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    alignments: Alignments,
    theme: &TableTheme,
    termwidth: usize,
) -> Option<String> {
    if table.is_empty {
        return None;
    }

    let table_data = &table.data;
    let with_header = table.with_header;
    let with_footer = with_header && need_footer(config, (&table.data).size().0 as u64);
    let with_index = !config.disable_table_indexes;

    let table: tabled::Table<RecordsInfo<'_, TextStyle>> =
        tabled::builder::Builder::custom(table_data.clone()).build();
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

fn print_table(table: tabled::Table<RecordsInfo<'_, TextStyle>>, config: &Config) -> String {
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
    table
        .lines()
        .next()
        .map_or(0, papergrid::util::string_width)
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

fn align_table<R>(
    mut table: tabled::Table<R>,
    alignments: Alignments,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
    data: &RecordsInfo<TextStyle>,
) -> tabled::Table<R>
where
    R: RecordsMut,
    for<'a> &'a R: Records,
{
    table = table.with(
        Modify::new(Segment::all())
            .with(Alignment::Horizontal(alignments.data))
            .with(AlignmentStrategy::PerLine),
    );

    if with_header {
        let alignment = Alignment::Horizontal(alignments.header);
        if with_footer {
            table = table.with(Modify::new(Rows::last()).with(alignment.clone()));
        }

        table = table.with(Modify::new(Rows::first()).with(alignment));
    }

    if with_index {
        table =
            table.with(Modify::new(Columns::first()).with(Alignment::Horizontal(alignments.index)));
    }

    table = override_alignments(table, data, with_header, with_index, alignments);

    table
}

fn override_alignments<R>(
    mut table: tabled::Table<R>,
    data: &RecordsInfo<TextStyle>,
    header_present: bool,
    index_present: bool,
    alignments: Alignments,
) -> tabled::Table<R>
where
    for<'a> &'a R: Records,
{
    let offset = if header_present { 1 } else { 0 };
    let (count_rows, count_columns) = data.size();
    for row in offset..count_rows {
        for col in 0..count_columns {
            let alignment = data[(row, col)].alignment;
            if index_present && col == 0 && alignment == alignments.index {
                continue;
            }

            if alignment == alignments.data {
                continue;
            }

            table = table.with(
                Cell(row, col)
                    .modify()
                    .with(Alignment::Horizontal(alignment)),
            );
        }
    }

    table
}

fn load_theme<R>(
    mut table: tabled::Table<R>,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
) -> tabled::Table<R>
where
    R: RecordsMut,
    for<'a> &'a R: Records,
{
    let mut theme = theme.theme.clone();
    if !with_header {
        theme.set_lines(HashMap::default());
    }

    table = table.with(theme);

    if let Some(color) = color_hm.get("separator") {
        let color = color.paint(" ").to_string();
        if let Ok(color) = Color::try_from(color) {
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

    table
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}

struct FooterStyle;

impl<R> TableOption<R> for FooterStyle
where
    for<'a> &'a R: Records,
{
    fn change(&mut self, table: &mut tabled::Table<R>) {
        if table.is_empty() {
            return;
        }

        if let Some(line) = table.get_config().get_split_line(1).cloned() {
            let count_rows = table.shape().0;
            table.get_config_mut().set_split_line(count_rows - 1, line);
        }
    }
}

fn table_trim_columns<'a>(
    table: tabled::Table<RecordsInfo<'a, TextStyle>>,
    termwidth: usize,
    trim_strategy: &TrimStrategy,
) -> tabled::Table<RecordsInfo<'a, TextStyle>> {
    table.with(&TrimStrategyModifier {
        termwidth,
        trim_strategy,
    })
}

pub struct TrimStrategyModifier<'a> {
    termwidth: usize,
    trim_strategy: &'a TrimStrategy,
}

impl<R> tabled::TableOption<R> for &TrimStrategyModifier<'_>
where
    R: RecordsMut,
    for<'a> &'a R: Records,
    for<'a> <&'a R as Records>::Cell: records::Cell,
{
    fn change(&mut self, table: &mut tabled::Table<R>) {
        match self.trim_strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let mut w = Width::wrap(self.termwidth).priority::<tabled::width::PriorityMax>();
                if *try_to_keep_words {
                    w = w.keep_words();
                }

                w.change(table)
            }
            TrimStrategy::Truncate { suffix } => {
                let mut w =
                    Width::truncate(self.termwidth).priority::<tabled::width::PriorityMax>();
                if let Some(suffix) = suffix {
                    w = w.suffix(suffix).suffix_try_color(true);
                }

                w.change(table);
            }
        };
    }
}

fn table_fix_lengths(headers: Option<&mut Vec<String>>, data: &mut [Vec<String>]) -> usize {
    let length = table_find_max_length(headers.as_deref(), data);

    if let Some(headers) = headers {
        headers.extend(iter::repeat(String::default()).take(length - headers.len()));
    }

    for row in data {
        row.extend(iter::repeat(String::default()).take(length - row.len()));
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

impl papergrid::Color for TextStyle {
    fn fmt_prefix(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(color) = &self.color_style {
            color.prefix().fmt(f)?;
        }

        Ok(())
    }

    fn fmt_suffix(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.color_style.is_some() {
            papergrid::Color::fmt_suffix(&(), f)?;
        }

        Ok(())
    }
}
