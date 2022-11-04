use std::{collections::HashMap, fmt::Display};

use nu_protocol::{Config, FooterMode, TrimStrategy};
use tabled::{
    alignment::AlignmentHorizontal,
    builder::Builder,
    color::Color,
    formatting::AlignmentStrategy,
    object::{Cell, Columns, Rows, Segment},
    papergrid::{
        self,
        records::{
            cell_info::CellInfo, tcell::TCell, vec_records::VecRecords, Records, RecordsMut,
        },
        width::CfgWidthFunction,
    },
    Alignment, Modify, ModifyObject, TableOption, Width,
};

use crate::{table_theme::TableTheme, TextStyle};

/// Table represent a table view.
#[derive(Debug)]
pub struct Table {
    data: Data,
    is_empty: bool,
    with_header: bool,
    with_index: bool,
}

type Data = VecRecords<TCell<CellInfo<'static>, TextStyle>>;

impl Table {
    /// Creates a [Table] instance.
    ///
    /// If `headers.is_empty` then no headers will be rendered.
    pub fn new(
        mut data: Vec<Vec<TCell<CellInfo<'static>, TextStyle>>>,
        size: (usize, usize),
        termwidth: usize,
        with_header: bool,
        with_index: bool,
    ) -> Table {
        // it's not guaranted that data will have all rows with the same number of columns.
        // but VecRecords::with_hint require this constrain.
        for row in &mut data {
            if row.len() < size.1 {
                row.extend(
                    std::iter::repeat(Self::create_cell(String::default(), TextStyle::default()))
                        .take(size.1 - row.len()),
                );
            }
        }

        let mut data = VecRecords::with_hint(data, size.1);
        let is_empty = maybe_truncate_columns(&mut data, size.1, termwidth);

        Table {
            data,
            is_empty,
            with_header,
            with_index,
        }
    }

    pub fn create_cell(text: String, style: TextStyle) -> TCell<CellInfo<'static>, TextStyle> {
        TCell::new(CellInfo::new(text, CfgWidthFunction::new(4)), style)
    }

    pub fn is_empty(&self) -> bool {
        self.is_empty
    }

    pub fn size(&self) -> (usize, usize) {
        (self.data.count_rows(), self.data.count_columns())
    }

    pub fn is_with_index(&self) -> bool {
        self.with_index
    }

    pub fn truncate(&mut self, width: usize, theme: &TableTheme) -> bool {
        let mut truncated = false;
        while self.data.count_rows() > 0 && self.data.count_columns() > 0 {
            let mut table = Builder::custom(self.data.clone()).build();
            load_theme(&mut table, &HashMap::new(), theme, false, false);
            let total = table.total_width();
            drop(table);

            if total > width {
                truncated = true;
                self.data.truncate(self.data.count_columns() - 1);
            } else {
                break;
            }
        }

        let is_empty = self.data.count_rows() == 0 || self.data.count_columns() == 0;
        if is_empty {
            return true;
        }

        if truncated {
            self.data.push(Table::create_cell(
                String::from("..."),
                TextStyle::default(),
            ));
        }

        false
    }

    /// Draws a trable on a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw_table(
        self,
        config: &Config,
        color_hm: &HashMap<String, nu_ansi_term::Style>,
        alignments: Alignments,
        theme: &TableTheme,
        termwidth: usize,
    ) -> Option<String> {
        draw_table(self, config, color_hm, alignments, theme, termwidth)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Alignments {
    pub(crate) data: AlignmentHorizontal,
    pub(crate) index: AlignmentHorizontal,
    pub(crate) header: AlignmentHorizontal,
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

fn draw_table(
    mut table: Table,
    config: &Config,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    alignments: Alignments,
    theme: &TableTheme,
    termwidth: usize,
) -> Option<String> {
    if table.is_empty {
        return None;
    }

    let with_header = table.with_header;
    let with_footer = with_header && need_footer(config, (table.data).size().0 as u64);
    let with_index = table.with_index;

    if with_footer {
        table.data.duplicate_row(0);
    }

    let mut table = Builder::custom(table.data).build();
    load_theme(&mut table, color_hm, theme, with_footer, with_header);
    align_table(&mut table, alignments, with_index, with_header, with_footer);
    table_trim_columns(&mut table, termwidth, &config.trim_strategy);

    let table = print_table(table, config);
    if table_width(&table) > termwidth {
        None
    } else {
        Some(table)
    }
}

fn print_table(table: tabled::Table<Data>, config: &Config) -> String {
    let output = table.to_string();

    // the atty is for when people do ls from vim, there should be no coloring there
    if !config.use_ansi_coloring || !atty::is(atty::Stream::Stdout) {
        // Draw the table without ansi colors
        nu_utils::strip_ansi_string_likely(output)
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

fn align_table(
    table: &mut tabled::Table<Data>,
    alignments: Alignments,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
) {
    table.with(
        Modify::new(Segment::all())
            .with(Alignment::Horizontal(alignments.data))
            .with(AlignmentStrategy::PerLine),
    );

    if with_header {
        let alignment = Alignment::Horizontal(alignments.header);
        if with_footer {
            table.with(Modify::new(Rows::last()).with(alignment.clone()));
        }

        table.with(Modify::new(Rows::first()).with(alignment));
    }

    if with_index {
        table.with(Modify::new(Columns::first()).with(Alignment::Horizontal(alignments.index)));
    }

    override_alignments(table, with_header, with_index, alignments);
}

fn override_alignments(
    table: &mut tabled::Table<Data>,
    header_present: bool,
    index_present: bool,
    alignments: Alignments,
) {
    let offset = usize::from(header_present);
    let (count_rows, count_columns) = table.shape();
    for row in offset..count_rows {
        for col in 0..count_columns {
            let alignment = table.get_records()[(row, col)].get_data().alignment;
            if index_present && col == 0 && alignment == alignments.index {
                continue;
            }

            if alignment == alignments.data {
                continue;
            }

            table.with(
                Cell(row, col)
                    .modify()
                    .with(Alignment::Horizontal(alignment)),
            );
        }
    }
}

fn load_theme<R>(
    table: &mut tabled::Table<R>,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
) where
    R: Records,
{
    let mut theme = theme.theme.clone();
    if !with_header {
        theme.set_horizontals(HashMap::default());
    }

    table.with(theme);

    if let Some(color) = color_hm.get("separator") {
        let color = color.paint(" ").to_string();
        if let Ok(color) = Color::try_from(color) {
            table.with(color);
        }
    }

    if with_footer {
        table.with(FooterStyle).with(
            Modify::new(Rows::last())
                .with(Alignment::center())
                .with(AlignmentStrategy::PerCell),
        );
    }
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}

struct FooterStyle;

impl<R> TableOption<R> for FooterStyle
where
    R: Records,
{
    fn change(&mut self, table: &mut tabled::Table<R>) {
        if table.is_empty() {
            return;
        }

        if let Some(line) = table.get_config().get_horizontal_line(1).cloned() {
            let count_rows = table.shape().0;
            table
                .get_config_mut()
                .set_horizontal_line(count_rows - 1, line);
        }
    }
}

fn table_trim_columns(
    table: &mut tabled::Table<Data>,
    termwidth: usize,
    trim_strategy: &TrimStrategy,
) {
    table.with(TrimStrategyModifier::new(termwidth, trim_strategy));
}

pub struct TrimStrategyModifier<'a> {
    termwidth: usize,
    trim_strategy: &'a TrimStrategy,
}

impl<'a> TrimStrategyModifier<'a> {
    pub fn new(termwidth: usize, trim_strategy: &'a TrimStrategy) -> Self {
        Self {
            termwidth,
            trim_strategy,
        }
    }
}

impl<R> tabled::TableOption<R> for TrimStrategyModifier<'_>
where
    R: Records + RecordsMut<String>,
{
    fn change(&mut self, table: &mut tabled::Table<R>) {
        match self.trim_strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let mut w = Width::wrap(self.termwidth).priority::<tabled::peaker::PriorityMax>();
                if *try_to_keep_words {
                    w = w.keep_words();
                }

                w.change(table)
            }
            TrimStrategy::Truncate { suffix } => {
                let mut w =
                    Width::truncate(self.termwidth).priority::<tabled::peaker::PriorityMax>();
                if let Some(suffix) = suffix {
                    w = w.suffix(suffix).suffix_try_color(true);
                }

                w.change(table);
            }
        };
    }
}

fn maybe_truncate_columns(data: &mut Data, length: usize, termwidth: usize) -> bool {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;
    if max_num_of_columns == 0 {
        return true;
    }

    // If we have too many columns, truncate the table
    if max_num_of_columns < length {
        data.truncate(max_num_of_columns);
        data.push(Table::create_cell(
            String::from("..."),
            TextStyle::default(),
        ));
    }

    false
}

impl papergrid::Color for TextStyle {
    fn fmt_prefix(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(color) = &self.color_style {
            color.prefix().fmt(f)?;
        }

        Ok(())
    }

    fn fmt_suffix(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(color) = &self.color_style {
            if !color.is_plain() {
                f.write_str("\u{1b}[0m")?;
            }
        }

        Ok(())
    }
}
