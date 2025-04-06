use std::{cmp::min, collections::HashMap};

use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::{TableIndent, TrimStrategy};

use tabled::{
    builder::Builder,
    grid::{
        ansi::ANSIBuf,
        config::{AlignmentHorizontal, ColoredConfig, Entity, Position},
        dimension::CompleteDimensionVecRecords,
        records::{
            vec_records::{Cell, Text, VecRecords},
            ExactRecords, Records,
        },
    },
    settings::{
        formatting::AlignmentStrategy,
        object::{Columns, Rows},
        peaker::Priority,
        themes::ColumnNames,
        width::Truncate,
        Alignment, Color, Padding, TableOption, Width,
    },
    Table,
};

use crate::{convert_style, is_color_empty, table_theme::TableTheme};

const EMPTY_COLUMN_TEXT: &str = "...";
const EMPTY_COLUMN_TEXT_WIDTH: usize = 3;

pub type NuRecords = VecRecords<NuRecordsValue>;
pub type NuRecordsValue = Text<String>;

/// NuTable is a table rendering implementation.
#[derive(Debug, Clone)]
pub struct NuTable {
    data: NuRecords,
    styles: Styles,
    alignments: Alignments,
    config: TableConfig,
}

impl NuTable {
    /// Creates an empty [`NuTable`] instance.
    pub fn new(count_rows: usize, count_columns: usize) -> Self {
        Self {
            data: VecRecords::new(vec![vec![Text::default(); count_columns]; count_rows]),
            styles: Styles::default(),
            alignments: Alignments {
                data: AlignmentHorizontal::Left,
                index: AlignmentHorizontal::Right,
                header: AlignmentHorizontal::Center,
                columns: HashMap::default(),
                cells: HashMap::default(),
            },
            config: TableConfig {
                theme: TableTheme::basic(),
                trim: TrimStrategy::truncate(None),
                structure: TableStructure::new(false, false, false),
                indent: TableIndent::new(1, 1),
                header_on_border: false,
                expand: false,
                border_color: None,
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
        self.data[pos.0][pos.1] = Text::new(text);
    }

    pub fn insert_row(&mut self, index: usize, row: Vec<String>) {
        let data = &mut self.data[index];

        for (col, text) in row.into_iter().enumerate() {
            data[col] = Text::new(text);
        }
    }

    pub fn set_row(&mut self, index: usize, row: Vec<NuRecordsValue>) {
        assert_eq!(self.data[index].len(), row.len());
        self.data[index] = row;
    }

    pub fn set_column_style(&mut self, column: usize, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.columns.insert(column, style);
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.alignments.data {
            self.alignments.columns.insert(column, alignment);
        }
    }

    pub fn insert_style(&mut self, pos: Position, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.cells.insert(pos, style);
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.alignments.data {
            self.alignments.cells.insert(pos, alignment);
        }
    }

    pub fn set_header_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.header = style;
        }

        self.alignments.header = convert_alignment(style.alignment);
    }

    pub fn set_index_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.index = style;
        }

        self.alignments.index = convert_alignment(style.alignment);
    }

    pub fn set_data_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.data = style;
        }

        self.alignments.data = convert_alignment(style.alignment);
    }

    pub fn set_indent(&mut self, indent: TableIndent) {
        self.config.indent = indent;
    }

    pub fn set_theme(&mut self, theme: TableTheme) {
        self.config.theme = theme;
    }

    pub fn set_structure(&mut self, index: bool, header: bool, footer: bool) {
        self.config.structure = TableStructure::new(index, header, footer);
    }

    pub fn set_border_header(&mut self, on: bool) {
        self.config.header_on_border = on;
    }

    pub fn set_trim(&mut self, strategy: TrimStrategy) {
        self.config.trim = strategy;
    }

    pub fn set_strategy(&mut self, expand: bool) {
        self.config.expand = expand;
    }

    pub fn set_border_color(&mut self, color: Style) {
        self.config.border_color = (!color.is_plain()).then_some(color);
    }

    pub fn get_records_mut(&mut self) -> &mut NuRecords {
        &mut self.data
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, termwidth: usize) -> Option<String> {
        build_table(self, termwidth)
    }

    /// Return a total table width.
    pub fn total_width(&self) -> usize {
        let config = create_config(&self.config.theme, false, None);
        let pad = indent_sum(self.config.indent);
        let widths = build_width(&self.data, pad);
        get_total_width2(&widths, &config)
    }
}

impl From<Vec<Vec<Text<String>>>> for NuTable {
    fn from(value: Vec<Vec<Text<String>>>) -> Self {
        let mut nutable = Self::new(0, 0);
        nutable.data = VecRecords::new(value);

        nutable
    }
}

type Alignments = CellConfiguration<AlignmentHorizontal>;

type Styles = CellConfiguration<Color>;

#[derive(Debug, Default, Clone)]
struct CellConfiguration<Value> {
    data: Value,
    index: Value,
    header: Value,
    columns: HashMap<usize, Value>,
    cells: HashMap<Position, Value>,
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    theme: TableTheme,
    trim: TrimStrategy,
    border_color: Option<Style>,
    expand: bool,
    structure: TableStructure,
    header_on_border: bool,
    indent: TableIndent,
}

#[derive(Debug, Clone, Copy)]
struct TableStructure {
    with_index: bool,
    with_header: bool,
    with_footer: bool,
}

impl TableStructure {
    fn new(with_index: bool, with_header: bool, with_footer: bool) -> Self {
        Self {
            with_index,
            with_header,
            with_footer,
        }
    }
}

#[derive(Debug, Clone)]
struct HeadInfo {
    values: Vec<String>,
    align: AlignmentHorizontal,
    color: Option<Color>,
}

impl HeadInfo {
    fn new(values: Vec<String>, align: AlignmentHorizontal, color: Option<Color>) -> Self {
        Self {
            values,
            align,
            color,
        }
    }
}

fn build_table(mut t: NuTable, termwidth: usize) -> Option<String> {
    if t.count_columns() == 0 || t.count_rows() == 0 {
        return Some(String::new());
    }

    let widths = table_truncate(&mut t, termwidth)?;
    let head = remove_header_if(&mut t);
    table_insert_footer_if(&mut t);

    draw_table(t, widths, head, termwidth)
}

fn remove_header_if(t: &mut NuTable) -> Option<HeadInfo> {
    if !is_header_on_border(t) {
        return None;
    }

    let head = remove_header(t);
    t.config.structure.with_header = false;

    Some(head)
}

fn is_header_on_border(t: &NuTable) -> bool {
    let is_configured = t.config.structure.with_header && t.config.header_on_border;
    let has_horizontal = t.config.theme.as_base().borders_has_top()
        || t.config.theme.as_base().get_horizontal_line(1).is_some();
    is_configured && has_horizontal
}

fn table_insert_footer_if(t: &mut NuTable) {
    if t.config.structure.with_header && t.config.structure.with_footer {
        duplicate_row(&mut t.data, 0);
    }
}

fn table_truncate(t: &mut NuTable, termwidth: usize) -> Option<Vec<usize>> {
    let widths = maybe_truncate_columns(&mut t.data, &t.config, termwidth);
    if widths.is_empty() {
        return None;
    }

    Some(widths)
}

fn remove_header(t: &mut NuTable) -> HeadInfo {
    let head: Vec<String> = t
        .data
        .remove(0)
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let align = t.alignments.header;
    let color = if is_color_empty(&t.styles.header) {
        None
    } else {
        Some(t.styles.header.clone())
    };

    // move settings by one row down
    t.alignments.cells = t
        .alignments
        .cells
        .drain()
        .filter(|(k, _)| k.0 != 0)
        .map(|(k, v)| ((k.0 - 1, k.1), v))
        .collect();
    t.alignments.header = AlignmentHorizontal::Center;

    // move settings by one row down
    t.styles.cells = t
        .styles
        .cells
        .drain()
        .filter(|(k, _)| k.0 != 0)
        .map(|(k, v)| ((k.0 - 1, k.1), v))
        .collect();
    t.styles.header = Color::empty();

    HeadInfo::new(head, align, color)
}

fn draw_table(
    t: NuTable,
    widths: Vec<usize>,
    head: Option<HeadInfo>,
    termwidth: usize,
) -> Option<String> {
    let mut structure = t.config.structure;
    structure.with_footer = structure.with_footer && head.is_none();
    let sep_color = t.config.border_color;

    let data: Vec<Vec<_>> = t.data.into();
    let mut table = Builder::from_vec(data).build();

    set_indent(&mut table, t.config.indent);
    load_theme(&mut table, &t.config.theme, &structure, sep_color);
    align_table(&mut table, t.alignments, &structure);
    colorize_table(&mut table, t.styles, &structure);
    truncate_table(&mut table, &t.config, widths, termwidth);
    table_set_border_header(
        &mut table,
        head,
        &t.config.theme,
        t.config.structure.with_footer,
    );

    table_to_string(table, termwidth)
}

fn table_set_border_header(
    table: &mut Table,
    head: Option<HeadInfo>,
    theme: &TableTheme,
    with_footer: bool,
) {
    let head = match head {
        Some(head) => head,
        None => return,
    };

    if !theme.as_base().borders_has_top() {
        let line = theme.as_base().get_horizontal_line(1);
        if let Some(line) = line.cloned() {
            table.get_config_mut().insert_horizontal_line(0, line);
            if with_footer {
                let last_row = table.count_rows();
                table
                    .get_config_mut()
                    .insert_horizontal_line(last_row, line);
            }
        };
    }

    if with_footer {
        let last_row = table.count_rows();
        table.with(SetLineHeaders::new(last_row, head.clone()));
    }

    table.with(SetLineHeaders::new(0, head));
}

fn truncate_table(table: &mut Table, cfg: &TableConfig, widths: Vec<usize>, termwidth: usize) {
    let trim = cfg.trim.clone();
    let ctrl = WidthCtrl::new(termwidth, widths, trim, cfg.expand);
    table.with(ctrl);
}

fn indent_sum(indent: TableIndent) -> usize {
    indent.left + indent.right
}

fn set_indent(table: &mut Table, indent: TableIndent) {
    table.with(Padding::new(indent.left, indent.right, 0, 0));
}

fn table_to_string(table: Table, termwidth: usize) -> Option<String> {
    let total_width = table.total_width();

    if total_width > termwidth {
        None
    } else {
        let content = table.to_string();
        Some(content)
    }
}

struct WidthCtrl {
    widths: Vec<usize>,
    trim_strategy: TrimStrategy,
    max_width: usize,
    expand: bool,
}

impl WidthCtrl {
    fn new(
        max_width: usize,
        widths: Vec<usize>,
        trim_strategy: TrimStrategy,
        expand: bool,
    ) -> Self {
        Self {
            widths,
            trim_strategy,
            max_width,
            expand,
        }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for WidthCtrl {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dims: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let total_width = get_total_width2(&self.widths, cfg);
        let need_truncation = total_width > self.max_width;
        if need_truncation {
            width_ctrl_truncate(self, recs, cfg, dims);
            return;
        }

        let need_expansion = self.expand;
        if need_expansion {
            width_ctrl_expand(self, recs, cfg, dims);
            return;
        }

        SetDimensions(self.widths).change(recs, cfg, dims);
    }
}

fn width_ctrl_expand(
    ctrl: WidthCtrl,
    recs: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords,
) {
    let opt = (SetDimensions(ctrl.widths), Width::increase(ctrl.max_width));
    TableOption::<VecRecords<_>, _, _>::change(opt, recs, cfg, dims);
}

fn width_ctrl_truncate(
    ctrl: WidthCtrl,
    recs: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords,
) {
    match ctrl.trim_strategy {
        TrimStrategy::Wrap { try_to_keep_words } => {
            let wrap = Width::wrap(ctrl.max_width)
                .keep_words(try_to_keep_words)
                .priority(Priority::max(false));

            let opt = (SetDimensions(ctrl.widths), wrap);
            TableOption::<NuRecords, _, _>::change(opt, recs, cfg, dims);
        }
        TrimStrategy::Truncate { suffix } => {
            let mut truncate = Width::truncate(ctrl.max_width).priority(Priority::max(false));
            if let Some(suffix) = suffix {
                truncate = truncate.suffix(suffix).suffix_try_color(true);
            }

            let opt = (SetDimensions(ctrl.widths), truncate);
            TableOption::<NuRecords, _, _>::change(opt, recs, cfg, dims);
        }
    }
}

fn align_table(table: &mut Table, alignments: Alignments, structure: &TableStructure) {
    table.with(AlignmentStrategy::PerLine);
    table.with(Alignment::from(alignments.data));

    for (column, alignment) in alignments.columns {
        table.modify(Columns::single(column), Alignment::from(alignment));
    }

    for (pos, alignment) in alignments.cells {
        table.modify(pos, Alignment::from(alignment));
    }

    if structure.with_header {
        table.modify(Rows::first(), Alignment::from(alignments.header));

        if structure.with_footer {
            table.modify(Rows::last(), Alignment::from(alignments.header));
        }
    }

    if structure.with_index {
        table.modify(Columns::first(), Alignment::from(alignments.index));
    }
}

fn colorize_table(table: &mut Table, styles: Styles, structure: &TableStructure) {
    if !is_color_empty(&styles.data) {
        table.with(styles.data);
    }

    for (column, color) in styles.columns {
        if !is_color_empty(&color) {
            table.modify(Columns::single(column), color);
        }
    }

    for (pos, color) in styles.cells {
        if !is_color_empty(&color) {
            table.modify(pos, color);
        }
    }

    if structure.with_index && !is_color_empty(&styles.index) {
        table.modify(Columns::first(), styles.index);
    }

    if structure.with_header && !is_color_empty(&styles.header) {
        table.modify(Rows::first(), styles.header.clone());
    }

    if structure.with_footer && !is_color_empty(&styles.header) {
        table.modify(Rows::last(), styles.header);
    }
}

fn load_theme(
    table: &mut Table,
    theme: &TableTheme,
    structure: &TableStructure,
    sep_color: Option<Style>,
) {
    let mut theme = theme.as_base().clone();

    if !structure.with_header {
        let borders = *theme.get_borders();
        theme.remove_horizontal_lines();
        theme.set_borders(borders);
    } else if structure.with_footer {
        theme_copy_horizontal_line(&mut theme, 1, table.count_rows() - 1);
    }

    table.with(theme);

    if let Some(style) = sep_color {
        let color = convert_style(style);
        let color = ANSIBuf::from(color);
        table.get_config_mut().set_border_color_default(color);
    }
}

fn maybe_truncate_columns(data: &mut NuRecords, cfg: &TableConfig, termwidth: usize) -> Vec<usize> {
    const TERMWIDTH_THRESHOLD: usize = 120;

    let pad = cfg.indent.left + cfg.indent.right;
    let preserve_content = termwidth > TERMWIDTH_THRESHOLD;

    if preserve_content {
        truncate_columns_by_columns(data, &cfg.theme, pad, termwidth)
    } else {
        truncate_columns_by_content(data, &cfg.theme, pad, termwidth)
    }
}

// VERSION where we are showing AS LITTLE COLUMNS AS POSSIBLE but WITH AS MUCH CONTENT AS POSSIBLE.
fn truncate_columns_by_content(
    data: &mut NuRecords,
    theme: &TableTheme,
    pad: usize,
    termwidth: usize,
) -> Vec<usize> {
    const MIN_ACCEPTABLE_WIDTH: usize = 3;
    const TRAILING_COLUMN_WIDTH: usize = 5;

    let config = create_config(theme, false, None);
    let mut widths = build_width(&*data, pad);
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
        widths.push(EMPTY_COLUMN_TEXT_WIDTH + pad);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(EMPTY_COLUMN_TEXT_WIDTH + pad);
    }

    widths
}

// VERSION where we are showing AS MANY COLUMNS AS POSSIBLE but as a side affect they MIGHT CONTAIN AS LITTLE CONTENT AS POSSIBLE
fn truncate_columns_by_columns(
    data: &mut NuRecords,
    theme: &TableTheme,
    pad: usize,
    termwidth: usize,
) -> Vec<usize> {
    let acceptable_width = 10 + pad;
    let trailing_column_width = 3 + pad;

    let config = create_config(theme, false, None);
    let mut widths = build_width(&*data, pad);
    let total_width = get_total_width2(&widths, &config);
    if total_width <= termwidth {
        return widths;
    }

    let widths_total = widths.iter().sum::<usize>();
    let min_widths = widths
        .iter()
        .map(|w| min(*w, acceptable_width))
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
        let width = min(widths[column], acceptable_width);
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
    if diff > trailing_column_width {
        push_empty_column(data);
        widths.push(EMPTY_COLUMN_TEXT_WIDTH + pad);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(EMPTY_COLUMN_TEXT_WIDTH + pad);
    }

    widths
}

fn get_total_width2(widths: &[usize], cfg: &ColoredConfig) -> usize {
    let total = widths.iter().sum::<usize>();
    let countv = cfg.count_vertical(widths.len());
    let margin = cfg.get_margin();

    total + countv + margin.left.size + margin.right.size
}

fn create_config(theme: &TableTheme, with_header: bool, color: Option<Style>) -> ColoredConfig {
    let structure = TableStructure::new(false, with_header, false);
    let mut table = Table::new([[""]]);
    load_theme(&mut table, theme, &structure, color);
    table.get_config().clone()
}

fn push_empty_column(data: &mut NuRecords) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    let empty_cell = Text::new(String::from(EMPTY_COLUMN_TEXT));
    for row in &mut inner {
        row.push(empty_cell.clone());
    }

    *data = VecRecords::new(inner);
}

fn duplicate_row(data: &mut NuRecords, row: usize) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    let duplicate = inner[row].clone();
    inner.push(duplicate);

    *data = VecRecords::new(inner);
}

fn truncate_columns(data: &mut NuRecords, count: usize) {
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

struct SetDimensions(Vec<usize>);

impl<R> TableOption<R, ColoredConfig, CompleteDimensionVecRecords<'_>> for SetDimensions {
    fn change(self, _: &mut R, _: &mut ColoredConfig, dims: &mut CompleteDimensionVecRecords<'_>) {
        dims.set_widths(self.0);
    }
}

// it assumes no spans is used.
// todo: Could be replaced by Dimension impl usage
fn build_width(records: &NuRecords, pad: usize) -> Vec<usize> {
    let count_columns = records.count_columns();
    let mut widths = vec![0; count_columns];
    for columns in records.iter_rows() {
        for (col, cell) in columns.iter().enumerate() {
            let width = Cell::width(cell) + pad;
            widths[col] = std::cmp::max(widths[col], width);
        }
    }

    widths
}

// It's laverages a use of guuaranted cached widths before hand
// to speed up things a bit.
struct SetLineHeaders {
    line: usize,
    head: HeadInfo,
}

impl SetLineHeaders {
    fn new(line: usize, head: HeadInfo) -> Self {
        Self { line, head }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for SetLineHeaders {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dims: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let widths = match dims.get_widths() {
            Some(widths) => widths,
            None => {
                // we don't have widths cached; which means that NO width adjustments were done
                // which means we are OK to leave columns as they are.
                //
                // but we actually always have to have widths at this point

                unreachable!("must never be the case");
            }
        };

        let columns: Vec<_> = self
            .head
            .values
            .into_iter()
            .zip(widths.iter().cloned()) // it must be always safe to do
            .map(|(s, width)| Truncate::truncate(&s, width).into_owned())
            .collect();

        let mut names = ColumnNames::new(columns)
            .line(self.line)
            .alignment(Alignment::from(self.head.align));
        if let Some(color) = self.head.color {
            names = names.color(color);
        }

        names.change(recs, cfg, dims);
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}

fn theme_copy_horizontal_line(theme: &mut tabled::settings::Theme, from: usize, to: usize) {
    if let Some(line) = theme.get_horizontal_line(from) {
        theme.insert_horizontal_line(to, *line);
    }
}

struct GetDims(Vec<usize>);

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for &mut GetDims {
    fn change(
        self,
        _: &mut NuRecords,
        _: &mut ColoredConfig,
        dims: &mut CompleteDimensionVecRecords<'_>,
    ) {
        self.0 = dims.get_widths().expect("expected to get it").to_vec();
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}
