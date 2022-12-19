use crate::table_theme::TableTheme;
use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::TrimStrategy;
use std::{cmp::min, collections::HashMap};
use tabled::{
    alignment::AlignmentHorizontal,
    builder::Builder,
    color::Color,
    formatting::AlignmentStrategy,
    object::{Cell, Columns, Rows, Segment},
    papergrid::{
        records::{
            cell_info::CellInfo, tcell::TCell, vec_records::VecRecords, Records, RecordsMut,
        },
        util::string_width_multiline,
        width::{CfgWidthFunction, WidthEstimator},
        Estimate,
    },
    peaker::Peaker,
    Alignment, Modify, ModifyObject, TableOption, Width,
};

/// Table represent a table view.
#[derive(Debug, Clone)]
pub struct Table {
    data: Data,
}

type Data = VecRecords<TCell<CellInfo<'static>, TextStyle>>;

impl Table {
    /// Creates a [Table] instance.
    ///
    /// If `headers.is_empty` then no headers will be rendered.
    pub fn new(data: Vec<Vec<TCell<CellInfo<'static>, TextStyle>>>, size: (usize, usize)) -> Table {
        // it's not guaranted that data will have all rows with the same number of columns.
        // but VecRecords::with_hint require this constrain.
        //
        // so we do a check to make it certainly true

        let mut data = data;
        make_data_consistent(&mut data, size);

        let data = VecRecords::with_hint(data, size.1);

        Table { data }
    }

    pub fn count_rows(&self) -> usize {
        self.data.count_rows()
    }

    pub fn create_cell(
        text: impl Into<String>,
        style: TextStyle,
    ) -> TCell<CellInfo<'static>, TextStyle> {
        TCell::new(CellInfo::new(text.into(), CfgWidthFunction::new(4)), style)
    }

    pub fn truncate(&mut self, width: usize, theme: &TableTheme) -> bool {
        let mut truncated = false;
        while self.data.count_rows() > 0 && self.data.count_columns() > 0 {
            let total;
            {
                let mut table = Builder::custom(self.data.clone()).build();
                load_theme(&mut table, theme, false, false, None);
                total = table.total_width();
            }

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

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, config: TableConfig, termwidth: usize) -> Option<String> {
        build_table(self.data, config, termwidth)
    }
}

fn make_data_consistent(data: &mut Vec<Vec<TCell<CellInfo, TextStyle>>>, size: (usize, usize)) {
    for row in data {
        if row.len() < size.1 {
            row.extend(
                std::iter::repeat(Table::create_cell(String::default(), TextStyle::default()))
                    .take(size.1 - row.len()),
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    theme: TableTheme,
    alignments: Alignments,
    trim: TrimStrategy,
    split_color: Option<Style>,
    expand: bool,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
}

impl TableConfig {
    pub fn new(
        theme: TableTheme,
        with_header: bool,
        with_index: bool,
        append_footer: bool,
    ) -> Self {
        Self {
            theme,
            with_header,
            with_index,
            with_footer: append_footer,
            expand: false,
            alignments: Alignments::default(),
            trim: TrimStrategy::truncate(None),
            split_color: None,
        }
    }

    pub fn expand(mut self) -> Self {
        self.expand = true;
        self
    }

    pub fn trim(mut self, strategy: TrimStrategy) -> Self {
        self.trim = strategy;
        self
    }

    pub fn splitline_style(mut self, color: Style) -> Self {
        self.split_color = Some(color);
        self
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

fn build_table(mut data: Data, cfg: TableConfig, termwidth: usize) -> Option<String> {
    let is_empty = maybe_truncate_columns(&mut data, &cfg.theme, termwidth);
    if is_empty {
        return None;
    }

    if cfg.with_footer {
        data.duplicate_row(0);
    }

    draw_table(
        data,
        &cfg.theme,
        cfg.alignments,
        cfg.with_index,
        cfg.with_header,
        cfg.with_footer,
        cfg.expand,
        cfg.split_color,
        &cfg.trim,
        termwidth,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_table(
    data: Data,
    theme: &TableTheme,
    alignments: Alignments,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
    expand: bool,
    split_color: Option<Style>,
    trim_strategy: &TrimStrategy,
    termwidth: usize,
) -> Option<String> {
    let mut table = Builder::custom(data).build();
    load_theme(&mut table, theme, with_footer, with_header, split_color);
    align_table(&mut table, alignments, with_index, with_header, with_footer);

    if expand {
        table.with(Width::increase(termwidth));
    }

    table_trim_columns(&mut table, termwidth, trim_strategy);

    let text = table.to_string();
    if string_width_multiline(&text) > termwidth {
        None
    } else {
        Some(text)
    }
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
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
    separator_color: Option<Style>,
) where
    R: Records,
{
    let mut theme = theme.theme.clone();
    if !with_header {
        theme.set_horizontals(HashMap::default());
    }

    table.with(theme);

    if let Some(color) = separator_color {
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
                let mut w = Width::wrap(self.termwidth).priority::<PriorityMax>();
                if *try_to_keep_words {
                    w = w.keep_words();
                }

                w.change(table)
            }
            TrimStrategy::Truncate { suffix } => {
                let mut w = Width::truncate(self.termwidth).priority::<PriorityMax>();
                if let Some(suffix) = suffix {
                    w = w.suffix(suffix).suffix_try_color(true);
                }

                w.change(table);
            }
        };
    }
}

fn maybe_truncate_columns(data: &mut Data, theme: &TableTheme, termwidth: usize) -> bool {
    const TERMWIDTH_TRESHHOLD: usize = 120;

    if data.count_columns() == 0 {
        return true;
    }

    let truncate = if termwidth > TERMWIDTH_TRESHHOLD {
        truncate_columns_by_columns
    } else {
        truncate_columns_by_content
    };

    truncate(data, theme, termwidth)
}

// VERSION where we are showing AS LITTLE COLUMNS AS POSSIBLE but WITH AS MUCH CONTENT AS POSSIBLE.
fn truncate_columns_by_content(data: &mut Data, theme: &TableTheme, termwidth: usize) -> bool {
    const MIN_ACCEPTABLE_WIDTH: usize = 3;
    const TRAILING_COLUMN_WIDTH: usize = 5;
    const TRAILING_COLUMN_STR: &str = "...";

    let config;
    let total;
    {
        let mut table = Builder::custom(&*data).build();
        load_theme(&mut table, theme, false, false, None);
        total = table.total_width();
        config = table.get_config().clone();
    }

    if total <= termwidth {
        return false;
    }

    let mut width_ctrl = WidthEstimator::default();
    width_ctrl.estimate(&*data, &config);
    let widths = Vec::from(width_ctrl);

    let borders = config.get_borders();
    let vertical_border_i = borders.has_vertical() as usize;

    let mut width = borders.has_left() as usize + borders.has_right() as usize;
    let mut truncate_pos = 0;
    for column_width in widths {
        width += column_width;
        width += vertical_border_i;

        if width >= termwidth {
            // check whether we CAN limit the column width
            width -= column_width;
            width += MIN_ACCEPTABLE_WIDTH;

            if width <= termwidth {
                truncate_pos += 1;
            }

            break;
        }

        truncate_pos += 1;
    }

    // we don't need any truncation then (is it possible?)
    if truncate_pos + 1 == data.count_columns() {
        return false;
    }

    if truncate_pos == 0 {
        return true;
    }

    data.truncate(truncate_pos);

    // Append columns with a trailing column

    let min_width = borders.has_left() as usize
        + borders.has_right() as usize
        + data.count_columns() * MIN_ACCEPTABLE_WIDTH
        + (data.count_columns() - 1) * vertical_border_i;

    let diff = termwidth - min_width;
    let can_be_squeezed = diff > TRAILING_COLUMN_WIDTH + vertical_border_i;

    if can_be_squeezed {
        let cell = Table::create_cell(String::from(TRAILING_COLUMN_STR), TextStyle::default());
        data.push(cell);
    } else {
        if data.count_columns() == 1 {
            return true;
        }

        data.truncate(data.count_columns() - 1);

        let cell = Table::create_cell(String::from(TRAILING_COLUMN_STR), TextStyle::default());
        data.push(cell);
    }

    false
}

// VERSION where we are showing AS MANY COLUMNS AS POSSIBLE but as a side affect they MIGHT CONTAIN AS LITTLE CONTENT AS POSSIBLE
fn truncate_columns_by_columns(data: &mut Data, theme: &TableTheme, termwidth: usize) -> bool {
    const ACCEPTABLE_WIDTH: usize = 10 + 2;
    const TRAILING_COLUMN_WIDTH: usize = 3 + 2;
    const TRAILING_COLUMN_STR: &str = "...";

    let config;
    let total;
    {
        let mut table = Builder::custom(&*data).build();
        load_theme(&mut table, theme, false, false, None);
        total = table.total_width();
        config = table.get_config().clone();
    }

    if total <= termwidth {
        return false;
    }

    let mut width_ctrl = WidthEstimator::default();
    width_ctrl.estimate(&*data, &config);
    let widths = Vec::from(width_ctrl);
    let widths_total = widths.iter().sum::<usize>();

    let min_widths = widths
        .iter()
        .map(|w| min(*w, ACCEPTABLE_WIDTH))
        .sum::<usize>();
    let mut min_total = total - widths_total + min_widths;

    if min_total <= termwidth {
        return false;
    }

    let mut i = 0;
    while data.count_columns() > 0 {
        i += 1;

        let column = data.count_columns() - 1 - i;
        let width = min(widths[column], ACCEPTABLE_WIDTH);
        min_total -= width;

        if config.get_borders().has_vertical() {
            min_total -= 1;
        }

        if min_total <= termwidth {
            break;
        }
    }

    if i + 1 == data.count_columns() {
        return true;
    }

    data.truncate(data.count_columns() - i);

    // Append columns with a trailing column
    let diff = termwidth - min_total;
    if diff > TRAILING_COLUMN_WIDTH {
        let cell = Table::create_cell(TRAILING_COLUMN_STR, TextStyle::default());
        data.push(cell);
    } else {
        if data.count_columns() == 1 {
            return true;
        }

        data.truncate(data.count_columns() - 1);

        let cell = Table::create_cell(TRAILING_COLUMN_STR, TextStyle::default());
        data.push(cell);
    }

    false
}

/// The same as [`tabled::peaker::PriorityMax`] but prioritizes left columns first in case of equal width.
#[derive(Debug, Default, Clone)]
pub struct PriorityMax;

impl Peaker for PriorityMax {
    fn create() -> Self {
        Self
    }

    fn peak(&mut self, _: &[usize], widths: &[usize]) -> Option<usize> {
        let col = (0..widths.len()).rev().max_by_key(|&i| widths[i]);
        match col {
            Some(col) => {
                if widths[col] == 0 {
                    None
                } else {
                    Some(col)
                }
            }
            None => None,
        }
    }
}
