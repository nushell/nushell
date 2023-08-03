use crate::table_theme::TableTheme;
use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::TrimStrategy;
use std::cmp::min;
use std::collections::HashMap;
use tabled::{
    builder::Builder,
    grid::{
        color::AnsiColor,
        config::{AlignmentHorizontal, ColoredConfig, Entity, EntityMap, Position},
        dimension::CompleteDimensionVecRecords,
        records::{
            vec_records::{CellInfo, VecRecords},
            ExactRecords, PeekableRecords, Records, Resizable,
        },
    },
    settings::{
        formatting::AlignmentStrategy, object::Segment, peaker::Peaker, themes::ColumnNames, Color,
        Modify, Settings, TableOption, Width,
    },
    Table,
};

/// NuTable is a table rendering implementation.
#[derive(Debug, Clone)]
pub struct NuTable {
    data: NuTableData,
    styles: Styles,
    alignments: Alignments,
}

type NuTableData = VecRecords<NuTableCell>;
pub type NuTableCell = CellInfo<String>;

#[derive(Debug, Default, Clone)]
struct Styles {
    index: AnsiColor<'static>,
    header: AnsiColor<'static>,
    data: EntityMap<AnsiColor<'static>>,
    data_is_set: bool,
}

#[derive(Debug, Clone)]
struct Alignments {
    data: AlignmentHorizontal,
    index: AlignmentHorizontal,
    header: AlignmentHorizontal,
    columns: HashMap<usize, AlignmentHorizontal>,
    cells: HashMap<Position, AlignmentHorizontal>,
}

impl NuTable {
    /// Creates an empty [Table] instance.
    pub fn new(count_rows: usize, count_columns: usize) -> Self {
        Self {
            data: VecRecords::new(vec![vec![CellInfo::default(); count_columns]; count_rows]),
            styles: Styles::default(),
            alignments: Alignments {
                data: AlignmentHorizontal::Left,
                index: AlignmentHorizontal::Right,
                header: AlignmentHorizontal::Center,
                columns: HashMap::default(),
                cells: HashMap::default(),
            },
        }
    }

    /// Return amount of rows.
    pub fn count_rows(&self) -> usize {
        self.data.count_rows()
    }

    /// Return amount of columns.
    pub fn count_columns(&self) -> usize {
        self.data.count_columns()
    }

    pub fn insert(&mut self, pos: Position, text: String) {
        self.data[pos.0][pos.1] = CellInfo::new(text);
    }

    pub fn set_column_style(&mut self, column: usize, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = AnsiColor::from(convert_style(style));
            self.styles.data.insert(Entity::Column(column), style);
            self.styles.data_is_set = true;
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.alignments.data {
            self.alignments.columns.insert(column, alignment);
        }
    }

    pub fn insert_style(&mut self, pos: Position, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = AnsiColor::from(convert_style(style));
            self.styles.data.insert(Entity::Cell(pos.0, pos.1), style);
            self.styles.data_is_set = true;
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.alignments.data {
            self.alignments.cells.insert(pos, alignment);
        }
    }

    pub fn set_header_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = AnsiColor::from(convert_style(style));
            self.styles.header = style;
        }

        self.alignments.header = convert_alignment(style.alignment);
    }

    pub fn set_index_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = AnsiColor::from(convert_style(style));
            self.styles.index = style;
        }

        self.alignments.index = convert_alignment(style.alignment);
    }

    pub fn set_data_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = AnsiColor::from(convert_style(style));
            self.styles.data.insert(Entity::Global, style);
            self.styles.data_is_set = true;
        }

        self.alignments.data = convert_alignment(style.alignment);
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, config: NuTableConfig, termwidth: usize) -> Option<String> {
        build_table(self.data, config, self.alignments, self.styles, termwidth)
    }

    /// Return a total table width.
    pub fn total_width(&self, config: &NuTableConfig) -> usize {
        let config = get_config(&config.theme, false, None);
        let widths = build_width(&self.data);
        get_total_width2(&widths, &config)
    }
}

impl From<Vec<Vec<CellInfo<String>>>> for NuTable {
    fn from(value: Vec<Vec<CellInfo<String>>>) -> Self {
        let mut nutable = Self::new(0, 0);
        nutable.data = VecRecords::new(value);

        nutable
    }
}

#[derive(Debug, Clone)]
pub struct NuTableConfig {
    pub theme: TableTheme,
    pub trim: TrimStrategy,
    pub split_color: Option<Style>,
    pub expand: bool,
    pub with_index: bool,
    pub with_header: bool,
    pub with_footer: bool,
    pub header_on_border: bool,
}

impl Default for NuTableConfig {
    fn default() -> Self {
        Self {
            theme: TableTheme::basic(),
            trim: TrimStrategy::truncate(None),
            with_header: false,
            with_index: false,
            with_footer: false,
            expand: false,
            split_color: None,
            header_on_border: false,
        }
    }
}

fn build_table(
    mut data: NuTableData,
    cfg: NuTableConfig,
    alignments: Alignments,
    styles: Styles,
    termwidth: usize,
) -> Option<String> {
    if data.count_columns() == 0 || data.count_rows() == 0 {
        return Some(String::new());
    }

    let widths = maybe_truncate_columns(&mut data, &cfg.theme, termwidth);
    if widths.is_empty() {
        return None;
    }

    if cfg.with_header && cfg.with_footer {
        duplicate_row(&mut data, 0);
    }

    draw_table(data, alignments, styles, widths, cfg, termwidth)
}

fn draw_table(
    data: NuTableData,
    mut alignments: Alignments,
    mut styles: Styles,
    widths: Vec<usize>,
    cfg: NuTableConfig,
    termwidth: usize,
) -> Option<String> {
    let data: Vec<Vec<_>> = data.into();
    let mut table = Builder::from(data).build();

    let need_header_move = cfg.header_on_border && has_horizontals_for_header(&cfg);
    let with_index = cfg.with_index;
    let with_header = cfg.with_header && table.count_rows() > 1 && !need_header_move;
    let with_footer = cfg.with_footer && !need_header_move;
    let sep_color = cfg.split_color;

    load_theme(&mut table, &cfg.theme, with_footer, with_header, sep_color);
    move_header_on_border(&mut table, &mut alignments, &mut styles, &cfg);
    align_table(&mut table, alignments, with_index, with_header, with_footer);
    colorize_table(&mut table, styles, with_index, with_header, with_footer);

    let total_width = get_total_width2(&widths, table.get_config());
    let total_width = if total_width > termwidth {
        table_trim_columns(&mut table, widths, termwidth, &cfg.trim);
        table.total_width()
    } else if cfg.expand && termwidth > total_width {
        table.with(Settings::new(
            SetDimensions(widths),
            Width::increase(termwidth),
        ));

        termwidth
    } else {
        total_width
    };

    if total_width > termwidth {
        None
    } else {
        let content = table.to_string();
        Some(content)
    }
}

fn move_header_on_border(
    table: &mut Table,
    alignments: &mut Alignments,
    styles: &mut Styles,
    cfg: &NuTableConfig,
) {
    if !cfg.header_on_border || table.count_rows() <= 1 {
        return;
    }

    let color = Color::from(styles.header.clone());
    let has_bottom_line = table
        .get_config()
        .has_horizontal(table.count_rows(), table.count_rows());
    let has_top_line = table.get_config().has_horizontal(0, table.count_rows());

    if cfg.with_header && has_top_line {
        move_row_on_border(table, 0, color.clone(), alignments.header)
    }

    if cfg.with_footer && has_bottom_line {
        let last_row = table.count_rows() - 1;
        move_row_on_border(table, last_row, color, alignments.header)
    }

    // because we remove rows we will invalidate the data alignments and colors
    // so we need to restore it back

    if cfg.with_header && has_top_line {
        if !alignments.cells.is_empty() {
            for row in 1..table.count_rows() {
                for col in 0..table.count_rows() {
                    let val = alignments.cells.get(&(row, col));
                    if let Some(val) = val {
                        alignments.cells.insert((row - 1, col), *val);
                    }
                }
            }
        }

        for row in 1..table.count_rows() {
            for col in 0..table.count_rows() {
                let val = styles.data.get(Entity::Cell(row, col)).clone();
                styles.data.insert(Entity::Cell(row - 1, col), val);
            }
        }
    }
}

fn align_table(
    table: &mut Table,
    alignments: Alignments,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
) {
    table
        .with(Modify::new(Segment::all()).with(AlignmentStrategy::PerLine))
        .with(SetAlignment(alignments.data, Entity::Global));

    for (column, alignment) in alignments.columns {
        table.with(SetAlignment(alignment, Entity::Column(column)));
    }

    for (pos, alignment) in alignments.cells {
        table.with(SetAlignment(alignment, Entity::Cell(pos.0, pos.1)));
    }

    if with_header {
        table.with(SetAlignment(alignments.header, Entity::Row(0)));

        if with_footer {
            table.with(SetAlignment(
                alignments.header,
                Entity::Row(table.count_rows() - 1),
            ));
        }
    }

    if with_index {
        table.with(SetAlignment(alignments.index, Entity::Column(0)));
    }
}

fn colorize_table(
    table: &mut Table,
    mut styles: Styles,
    with_index: bool,
    with_header: bool,
    with_footer: bool,
) {
    if with_index {
        styles.data.insert(Entity::Column(0), styles.index);
        styles.data_is_set = true;
    }

    if with_header {
        styles.data.insert(Entity::Row(0), styles.header.clone());
        styles.data_is_set = true;

        if with_footer {
            let count_rows = table.count_rows();
            if count_rows > 1 {
                let last_row = count_rows - 1;
                styles.data.insert(Entity::Row(last_row), styles.header);
            }
        }
    }

    if styles.data_is_set {
        table.get_config_mut().set_colors(styles.data);
    }
}

fn load_theme(
    table: &mut Table,
    theme: &TableTheme,
    with_footer: bool,
    with_header: bool,
    sep_color: Option<Style>,
) {
    let mut theme = theme.get_theme();

    if !with_header {
        theme.set_horizontals(std::collections::HashMap::new());
    } else if with_footer && table.count_rows() > 2 {
        if let Some(line) = theme.get_horizontal(1) {
            theme.insert_horizontal(table.count_rows() - 1, line);
        }
    }

    table.with(theme);

    if let Some(style) = sep_color {
        let color = convert_style(style);
        let color = AnsiColor::from(color);
        table.get_config_mut().set_border_color_global(color);
    }
}

struct FooterStyle;

impl<R: ExactRecords, D> TableOption<R, D, ColoredConfig> for FooterStyle {
    fn change(self, records: &mut R, cfg: &mut ColoredConfig, _: &mut D) {
        if let Some(line) = cfg.get_horizontal_line(1).cloned() {
            let count_rows = records.count_rows();
            cfg.insert_horizontal_line(count_rows - 1, line);
        }
    }
}

fn table_trim_columns(
    table: &mut Table,
    widths: Vec<usize>,
    termwidth: usize,
    trim_strategy: &TrimStrategy,
) {
    match trim_strategy {
        TrimStrategy::Wrap { try_to_keep_words } => {
            let mut wrap = Width::wrap(termwidth).priority::<PriorityMax>();
            if *try_to_keep_words {
                wrap = wrap.keep_words();
            }

            table.with(Settings::new(SetDimensions(widths), wrap));
        }
        TrimStrategy::Truncate { suffix } => {
            let mut truncate = Width::truncate(termwidth).priority::<PriorityMax>();
            if let Some(suffix) = suffix {
                truncate = truncate.suffix(suffix).suffix_try_color(true);
            }

            table.with(Settings::new(SetDimensions(widths), truncate));
        }
    }
}

fn maybe_truncate_columns(
    data: &mut NuTableData,
    theme: &TableTheme,
    termwidth: usize,
) -> Vec<usize> {
    const TERMWIDTH_THRESHOLD: usize = 120;

    let truncate = if termwidth > TERMWIDTH_THRESHOLD {
        truncate_columns_by_columns
    } else {
        truncate_columns_by_content
    };

    truncate(data, theme, termwidth)
}

// VERSION where we are showing AS LITTLE COLUMNS AS POSSIBLE but WITH AS MUCH CONTENT AS POSSIBLE.
fn truncate_columns_by_content(
    data: &mut NuTableData,
    theme: &TableTheme,
    termwidth: usize,
) -> Vec<usize> {
    const MIN_ACCEPTABLE_WIDTH: usize = 3;
    const TRAILING_COLUMN_WIDTH: usize = 5;

    let config = get_config(theme, false, None);
    let mut widths = build_width(&*data);
    let total_width = get_total_width2(&widths, &config);
    if total_width <= termwidth {
        return widths;
    }

    let borders = config.get_borders();
    let vertical_border_i = borders.has_vertical() as usize;

    let mut width = borders.has_left() as usize + borders.has_right() as usize;
    let mut truncate_pos = 0;
    for column_width in &widths {
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
    if truncate_pos == data.count_columns() {
        return widths;
    }

    if truncate_pos == 0 {
        return vec![];
    }

    truncate_columns(data, truncate_pos);
    widths.truncate(truncate_pos);

    // Append columns with a trailing column

    let min_width = borders.has_left() as usize
        + borders.has_right() as usize
        + data.count_columns() * MIN_ACCEPTABLE_WIDTH
        + (data.count_columns() - 1) * vertical_border_i;

    let diff = termwidth - min_width;
    let can_be_squeezed = diff > TRAILING_COLUMN_WIDTH + vertical_border_i;

    if can_be_squeezed {
        push_empty_column(data);
        widths.push(3 + 2);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(3 + 2);
    }

    widths
}

// VERSION where we are showing AS MANY COLUMNS AS POSSIBLE but as a side affect they MIGHT CONTAIN AS LITTLE CONTENT AS POSSIBLE
fn truncate_columns_by_columns(
    data: &mut NuTableData,
    theme: &TableTheme,
    termwidth: usize,
) -> Vec<usize> {
    const ACCEPTABLE_WIDTH: usize = 10 + 2;
    const TRAILING_COLUMN_WIDTH: usize = 3 + 2;

    let config = get_config(theme, false, None);
    let mut widths = build_width(&*data);
    let total_width = get_total_width2(&widths, &config);
    if total_width <= termwidth {
        return widths;
    }

    let widths_total = widths.iter().sum::<usize>();
    let min_widths = widths
        .iter()
        .map(|w| min(*w, ACCEPTABLE_WIDTH))
        .sum::<usize>();
    let mut min_total = total_width - widths_total + min_widths;

    if min_total <= termwidth {
        return widths;
    }

    // todo: simplify the loop
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
        return vec![];
    }

    truncate_columns(data, data.count_columns() - i);
    widths.pop();

    // Append columns with a trailing column
    let diff = termwidth - min_total;
    if diff > TRAILING_COLUMN_WIDTH {
        push_empty_column(data);
        widths.push(3 + 2);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(3 + 2);
    }

    widths
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
        col.filter(|&col| widths[col] != 0)
    }
}

fn get_total_width2(widths: &[usize], cfg: &ColoredConfig) -> usize {
    let total = widths.iter().sum::<usize>();
    let countv = cfg.count_vertical(widths.len());
    let margin = cfg.get_margin();

    total + countv + margin.left.size + margin.right.size
}

fn get_config(theme: &TableTheme, with_header: bool, color: Option<Style>) -> ColoredConfig {
    let mut table = Table::new([[""]]);
    load_theme(&mut table, theme, false, with_header, color);
    table.get_config().clone()
}

fn push_empty_column(data: &mut NuTableData) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    let empty_cell = CellInfo::new(String::from("..."));
    for row in &mut inner {
        row.push(empty_cell.clone());
    }

    *data = VecRecords::new(inner);
}

fn duplicate_row(data: &mut NuTableData, row: usize) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    let duplicate = inner[row].clone();
    inner.push(duplicate);

    *data = VecRecords::new(inner);
}

fn truncate_columns(data: &mut NuTableData, count: usize) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    for row in &mut inner {
        row.truncate(count);
    }

    *data = VecRecords::new(inner);
}

fn convert_alignment(alignment: nu_color_config::Alignment) -> AlignmentHorizontal {
    match alignment {
        nu_color_config::Alignment::Center => AlignmentHorizontal::Center,
        nu_color_config::Alignment::Left => AlignmentHorizontal::Left,
        nu_color_config::Alignment::Right => AlignmentHorizontal::Right,
    }
}

struct SetAlignment(AlignmentHorizontal, Entity);

impl<R, D> TableOption<R, D, ColoredConfig> for SetAlignment {
    fn change(self, _: &mut R, cfg: &mut ColoredConfig, _: &mut D) {
        cfg.set_alignment_horizontal(self.1, self.0);
    }
}

fn convert_style(style: Style) -> Color {
    Color::new(style.prefix().to_string(), style.suffix().to_string())
}

struct SetDimensions(Vec<usize>);

impl<R> TableOption<R, CompleteDimensionVecRecords<'_>, ColoredConfig> for SetDimensions {
    fn change(self, _: &mut R, _: &mut ColoredConfig, dims: &mut CompleteDimensionVecRecords<'_>) {
        dims.set_widths(self.0);
    }
}

// it assumes no spans is used.
fn build_width(records: &VecRecords<CellInfo<String>>) -> Vec<usize> {
    use tabled::grid::records::vec_records::Cell;
    const PAD: usize = 2;

    let count_columns = records.count_columns();
    let mut widths = vec![0; count_columns];
    for columns in records.iter_rows() {
        for (col, cell) in columns.iter().enumerate() {
            let width = Cell::width(cell) + PAD;
            widths[col] = std::cmp::max(widths[col], width);
        }
    }

    widths
}

fn move_row_on_border(table: &mut Table, row: usize, color: Color, alignment: AlignmentHorizontal) {
    if table.is_empty() {
        return;
    }

    let columns = (0..table.get_records().count_columns())
        .map(|column| table.get_records().get_text((row, column)).to_owned())
        .collect::<Vec<_>>();

    table.get_records_mut().remove_row(row);

    if table.is_empty() {
        table.get_records_mut().push_row();
    }

    let mut line = 0;
    if row != 0 {
        line = row;
    }

    let colors = std::iter::repeat(color)
        .take(table.count_columns())
        .collect::<Vec<_>>();
    let names = ColumnNames::new(columns)
        .set_line(line)
        .set_colors(colors)
        .set_alignment(alignment);

    table.with(names);
}

fn has_horizontals_for_header(cfg: &NuTableConfig) -> bool {
    cfg.theme.get_theme().get_horizontal(0).is_some()
        || cfg.theme.get_theme().get_borders().has_horizontal()
}
