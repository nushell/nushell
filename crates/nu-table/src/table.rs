use std::{cmp::min, collections::HashMap};

use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::{TableIndent, TrimStrategy};
use nu_utils::strip_ansi_unlikely;

use tabled::{
    builder::Builder,
    grid::{
        ansi::ANSIBuf,
        colors::Colors,
        config::{AlignmentHorizontal, ColoredConfig, Entity, Position},
        dimension::CompleteDimensionVecRecords,
        records::{
            vec_records::{Cell, Text, VecRecords},
            ExactRecords, Records, Resizable,
        },
    },
    settings::{
        format::FormatContent,
        formatting::AlignmentStrategy,
        object::{Columns, Row, Rows},
        peaker::Priority,
        themes::ColumnNames,
        width::Truncate,
        Alignment, Color, Format, Modify, ModifyList, Padding, Settings, TableOption, Width,
    },
    Table,
};

use crate::{convert_style, is_color_empty, table_theme::TableTheme};

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

#[derive(Debug, Clone)]
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

fn build_table(mut t: NuTable, termwidth: usize) -> Option<String> {
    if t.count_columns() == 0 || t.count_rows() == 0 {
        return Some(String::new());
    }

    let widths = table_truncate(&mut t, termwidth)?;
    table_insert_footer(&mut t);
    draw_table(t, widths, termwidth)
}

fn table_insert_footer(t: &mut NuTable) {
    if t.config.structure.with_header && t.config.structure.with_footer {
        duplicate_row(&mut t.data, 0);
    }
}

fn table_truncate(t: &mut NuTable, termwidth: usize) -> Option<Vec<usize>> {
    let pad = t.config.indent.left + t.config.indent.right;
    let widths = maybe_truncate_columns(&mut t.data, &t.config, termwidth, pad);
    if widths.is_empty() {
        return None;
    }

    Some(widths)
}

fn draw_table(t: NuTable, widths: Vec<usize>, termwidth: usize) -> Option<String> {
    let structure = get_table_structure(&t.data, &t.config);
    let sep_color = t.config.border_color;
    let border_header = structure.with_header && t.config.header_on_border;

    let data: Vec<Vec<_>> = t.data.into();
    let mut table = Builder::from_vec(data).build();

    set_indent(&mut table, t.config.indent);
    load_theme(&mut table, &t.config.theme, &structure, sep_color);
    align_table(&mut table, t.alignments, &structure);
    colorize_table(&mut table, t.styles, &structure);

    let pad = indent_sum(t.config.indent);
    let width_ctrl = WidthCtrl::new(widths, t.config, termwidth, pad);

    adjust_table(&mut table, width_ctrl, border_header, structure.with_footer);

    table_to_string(table, termwidth)
}

fn indent_sum(indent: TableIndent) -> usize {
    indent.left + indent.right
}

fn get_table_structure(data: &VecRecords<Text<String>>, cfg: &TableConfig) -> TableStructure {
    let with_index = cfg.structure.with_index;
    let with_header = cfg.structure.with_header && data.count_rows() > 1;
    let with_footer = with_header && cfg.structure.with_footer;

    TableStructure::new(with_index, with_header, with_footer)
}

fn adjust_table(table: &mut Table, width_ctrl: WidthCtrl, border_header: bool, with_footer: bool) {
    if border_header {
        if with_footer {
            set_border_head_with_footer(table, width_ctrl);
        } else {
            set_border_head(table, width_ctrl);
        }
    } else {
        table.with(width_ctrl);
    }
}

fn set_indent(table: &mut Table, indent: TableIndent) {
    table.with(Padding::new(indent.left, indent.right, 0, 0));
}

fn set_border_head(table: &mut Table, wctrl: WidthCtrl) {
    let mut row = GetRow(0, Vec::new());
    let mut row_opts = GetRowSettings(0, AlignmentHorizontal::Left, None);

    table.with(&mut row);
    table.with(&mut row_opts);

    table.with(
        Settings::default()
            .with(strip_color_from_row(0))
            .with(wctrl)
            .with(MoveRowNext::new(0, 0))
            .with(SetLineHeaders::new(0, row.1, row_opts.1, row_opts.2)),
    );
}

fn set_border_head_with_footer(table: &mut Table, wctrl: WidthCtrl) {
    // note: funnily last and row must be equal at this point but we do not rely on it just in case.

    let count_rows = table.count_rows();
    let last_row_index = count_rows - 1;

    let mut first_row = GetRow(0, Vec::new());
    let mut head_settings = GetRowSettings(0, AlignmentHorizontal::Left, None);
    let mut last_row = GetRow(last_row_index, Vec::new());

    table.with(&mut first_row);
    table.with(&mut head_settings);
    table.with(&mut last_row);

    let head = first_row.1;
    let footer = last_row.1;
    let alignment = head_settings.1;
    let head_color = head_settings.2.clone();
    let footer_color = head_settings.2;

    table.with(
        Settings::default()
            .with(strip_color_from_row(0))
            .with(strip_color_from_row(count_rows - 1))
            .with(wctrl)
            .with(MoveRowNext::new(0, 0))
            .with(MoveRowPrev::new(last_row_index - 1, last_row_index))
            .with(SetLineHeaders::new(0, head, alignment, head_color))
            .with(SetLineHeaders::new(
                last_row_index - 1,
                footer,
                alignment,
                footer_color,
            )),
    );
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
    width: Vec<usize>,
    cfg: TableConfig,
    width_max: usize,
    pad: usize,
}

impl WidthCtrl {
    fn new(width: Vec<usize>, cfg: TableConfig, max: usize, pad: usize) -> Self {
        Self {
            width,
            cfg,
            width_max: max,
            pad,
        }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for WidthCtrl {
    fn change(
        self,
        rec: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dim: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let total_width = get_total_width2(&self.width, cfg);

        let need_truncation = total_width > self.width_max;
        if need_truncation {
            let has_header = self.cfg.structure.with_header && rec.count_rows() > 1;
            let as_head = has_header && self.cfg.header_on_border;

            let trim = TableTrim::new(self.width, self.width_max, self.cfg.trim, as_head, self.pad);
            trim.change(rec, cfg, dim);
            return;
        }

        let need_expansion = self.cfg.expand && self.width_max > total_width;
        if need_expansion {
            let opt = (SetDimensions(self.width), Width::increase(self.width_max));
            TableOption::<VecRecords<_>, _, _>::change(opt, rec, cfg, dim);
            return;
        }

        SetDimensions(self.width).change(rec, cfg, dim);
    }
}

struct TableTrim {
    width: Vec<usize>,
    width_max: usize,
    strategy: TrimStrategy,
    trim_as_head: bool,
    pad: usize,
}

impl TableTrim {
    fn new(
        width: Vec<usize>,
        width_max: usize,
        strategy: TrimStrategy,
        trim_as_head: bool,
        pad: usize,
    ) -> Self {
        Self {
            width,
            strategy,
            pad,
            width_max,
            trim_as_head,
        }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for TableTrim {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dims: &mut CompleteDimensionVecRecords<'_>,
    ) {
        // we already must have been estimated that it's safe to do.
        // and all dims will be suffitient
        if self.trim_as_head {
            trim_as_header(recs, cfg, dims, self);
            return;
        }

        match self.strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let wrap = Width::wrap(self.width_max)
                    .keep_words(try_to_keep_words)
                    .priority(Priority::max(false));

                let opt = (SetDimensions(self.width), wrap);
                TableOption::<NuRecords, _, _>::change(opt, recs, cfg, dims);
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(self.width_max).priority(Priority::max(false));
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                let opt = (SetDimensions(self.width), truncate);
                TableOption::<NuRecords, _, _>::change(opt, recs, cfg, dims);
            }
        }
    }
}

fn trim_as_header(
    recs: &mut VecRecords<Text<String>>,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords,
    trim: TableTrim,
) {
    if recs.is_empty() {
        return;
    }

    let headers = recs[0].to_owned();
    let headers_widths = headers
        .iter()
        .map(Text::width)
        .map(|v| v + trim.pad)
        .collect::<Vec<_>>();
    let min_width_use = get_total_width2(&headers_widths, cfg);
    let mut free_width = trim.width_max.saturating_sub(min_width_use);

    // even though it's safe to trim columns by header there might be left unused space
    // so we do use it if possible prioritizing left columns
    let mut widths = vec![0; headers_widths.len()];
    for (i, head_width) in headers_widths.into_iter().enumerate() {
        let column_width = trim.width[i]; // safe to assume width is bigger then padding

        let mut use_width = head_width;
        if free_width > 0 {
            // it's safe to assume that column_width is always bigger or equal to head_width
            debug_assert!(column_width >= head_width);

            let additional_width = min(free_width, column_width - head_width);
            free_width -= additional_width;
            use_width += additional_width;
        }

        widths[i] = use_width;
    }

    // make sure we are fit in;
    // which is might not be the case where we need to truncate columns further then a column head width
    let expected_width = get_total_width2(&widths, cfg);
    if expected_width > trim.width_max {
        let mut diff = expected_width - trim.width_max;
        'out: loop {
            let (biggest_column, &value) = widths
                .iter()
                .enumerate()
                .max_by_key(|(_, &value)| value)
                .expect("ok");
            if value <= trim.pad {
                unreachable!("theoretically must never happen at this point")
            }

            widths[biggest_column] -= 1;
            diff -= 1;

            if diff == 0 {
                break 'out;
            }
        }
    }

    for (i, width) in widths.iter().cloned().enumerate() {
        let width = width - trim.pad;

        match &trim.strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let wrap = Width::wrap(width).keep_words(*try_to_keep_words);

                let opt = Modify::new(Columns::single(i)).with(wrap);
                TableOption::<VecRecords<_>, _, _>::change(opt, recs, cfg, dims);
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(width);
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                let opt = Modify::new(Columns::single(i)).with(truncate);
                TableOption::<VecRecords<_>, _, _>::change(opt, recs, cfg, dims);
            }
        }
    }

    TableOption::change(SetDimensions(widths), recs, cfg, dims);
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

fn maybe_truncate_columns(
    data: &mut NuRecords,
    cfg: &TableConfig,
    termwidth: usize,
    pad: usize,
) -> Vec<usize> {
    const TERMWIDTH_THRESHOLD: usize = 120;

    let preserve_content = termwidth > TERMWIDTH_THRESHOLD;
    let has_header = cfg.structure.with_header && data.count_rows() > 1;
    let is_header_on_border = has_header && cfg.header_on_border;

    let truncate = if is_header_on_border {
        truncate_columns_by_head
    } else if preserve_content {
        truncate_columns_by_columns
    } else {
        truncate_columns_by_content
    };

    truncate(data, &cfg.theme, pad, termwidth)
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
        widths.push(3 + pad);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(3 + pad);
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
        widths.push(3 + pad);
    } else {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        push_empty_column(data);
        widths.pop();
        widths.push(3 + pad);
    }

    widths
}

// VERSION where we are showing AS LITTLE COLUMNS AS POSSIBLE but WITH AS MUCH CONTENT AS POSSIBLE.
fn truncate_columns_by_head(
    data: &mut NuRecords,
    theme: &TableTheme,
    pad: usize,
    termwidth: usize,
) -> Vec<usize> {
    const TRAILING_COLUMN_WIDTH: usize = 5;

    let config = create_config(theme, false, None);
    let mut widths = build_width(&*data, pad);
    let total_width = get_total_width2(&widths, &config);
    if total_width <= termwidth {
        return widths;
    }

    if data.is_empty() {
        return widths;
    }

    let head = &data[0];

    let borders = config.get_borders();
    let has_vertical = borders.has_vertical();

    let mut width = borders.has_left() as usize + borders.has_right() as usize;
    let mut truncate_pos = 0;
    for (i, column_header) in head.iter().enumerate() {
        let column_header_width = Cell::width(column_header);
        width += column_header_width + pad;

        if i > 0 {
            width += has_vertical as usize;
        }

        if width >= termwidth {
            width -= column_header_width + (i > 0 && has_vertical) as usize + pad;
            break;
        }

        truncate_pos += 1;
    }

    // we don't need any truncation then (is it possible?)
    if truncate_pos == head.len() {
        return widths;
    }

    if truncate_pos == 0 {
        return vec![];
    }

    truncate_columns(data, truncate_pos);
    widths.truncate(truncate_pos);

    // Append columns with a trailing column

    let min_width = width;

    let diff = termwidth - min_width;
    let can_trailing_column_be_pushed = diff > TRAILING_COLUMN_WIDTH + has_vertical as usize;

    if !can_trailing_column_be_pushed {
        if data.count_columns() == 1 {
            return vec![];
        }

        truncate_columns(data, data.count_columns() - 1);
        widths.pop();
    }

    push_empty_column(data);
    widths.push(3 + pad);

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

    let empty_cell = Text::new(String::from("..."));
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

struct GetRow(usize, Vec<String>);

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for &mut GetRow {
    fn change(
        self,
        recs: &mut NuRecords,
        _: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let row = self.0;
        self.1 = recs[row].iter().map(|c| c.as_ref().to_owned()).collect();
    }
}

struct GetRowSettings(usize, AlignmentHorizontal, Option<Color>);

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>>
    for &mut GetRowSettings
{
    fn change(
        self,
        _: &mut NuRecords,
        cfg: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let row = self.0;
        self.1 = *cfg.get_alignment_horizontal(Entity::Row(row));
        self.2 = cfg
            .get_colors()
            .get_color((row, 0))
            .cloned()
            .map(Color::from);
    }
}

// It's laverages a use of guuaranted cached widths before hand
// to speed up things a bit.
struct SetLineHeaders {
    line: usize,
    columns: Vec<String>,
    alignment: AlignmentHorizontal,
    color: Option<Color>,
}

impl SetLineHeaders {
    fn new(
        line: usize,
        columns: Vec<String>,
        alignment: AlignmentHorizontal,
        color: Option<Color>,
    ) -> Self {
        Self {
            line,
            columns,
            alignment,
            color,
        }
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
            .columns
            .into_iter()
            .zip(widths.iter().cloned()) // it must be always safe to do
            .map(|(s, width)| Truncate::truncate(&s, width).into_owned())
            .collect();

        let mut names = ColumnNames::new(columns)
            .line(self.line)
            .alignment(Alignment::from(self.alignment));
        if let Some(color) = self.color {
            names = names.color(color);
        }

        names.change(recs, cfg, dims);
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}

struct MoveRowNext {
    row: usize,
    line: usize,
}

impl MoveRowNext {
    fn new(row: usize, line: usize) -> Self {
        Self { row, line }
    }
}

struct MoveRowPrev {
    row: usize,
    line: usize,
}

impl MoveRowPrev {
    fn new(row: usize, line: usize) -> Self {
        Self { row, line }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for MoveRowNext {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        row_shift_next(recs, cfg, self.row, self.line);
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimensionVecRecords<'_>> for MoveRowPrev {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        row_shift_prev(recs, cfg, self.row, self.line);
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}

fn row_shift_next(recs: &mut NuRecords, cfg: &mut ColoredConfig, row: usize, line: usize) {
    let count_rows = recs.count_rows();
    let count_columns = recs.count_columns();
    let has_line = cfg.has_horizontal(line, count_rows);
    let has_next_line = cfg.has_horizontal(line + 1, count_rows);
    if !has_line && !has_next_line {
        return;
    }

    recs.remove_row(row);
    let count_rows = recs.count_rows();

    shift_alignments_down(cfg, row, count_rows, count_columns);
    shift_colors_down(cfg, row, count_rows, count_columns);

    if !has_line {
        shift_lines_up(cfg, count_rows, &[line + 1]);
    } else {
        remove_lines(cfg, count_rows, &[line + 1]);
    }

    shift_lines_up(cfg, count_rows, &[count_rows]);
}

fn row_shift_prev(recs: &mut NuRecords, cfg: &mut ColoredConfig, row: usize, line: usize) {
    let mut count_rows = recs.count_rows();
    let count_columns = recs.count_columns();
    let has_line = cfg.has_horizontal(line, count_rows);
    let has_prev_line = cfg.has_horizontal(line - 1, count_rows);
    if !has_line && !has_prev_line {
        return;
    }

    recs.remove_row(row);

    if !has_line {
        return;
    }

    count_rows -= 1;

    shift_alignments_down(cfg, row, count_rows, count_columns);
    shift_colors_down(cfg, row, count_rows, count_columns);
    remove_lines(cfg, count_rows, &[line - 1]);
}

fn remove_lines(cfg: &mut ColoredConfig, count_rows: usize, line: &[usize]) {
    for &line in line {
        cfg.remove_horizontal_line(line, count_rows)
    }
}

fn shift_alignments_down(
    cfg: &mut ColoredConfig,
    row: usize,
    count_rows: usize,
    count_columns: usize,
) {
    for row in row..count_rows {
        for col in 0..count_columns {
            let pos = (row + 1, col).into();
            let posn = (row, col).into();
            let align = *cfg.get_alignment_horizontal(pos);
            cfg.set_alignment_horizontal(posn, align);
        }

        let align = *cfg.get_alignment_horizontal(Entity::Row(row + 1));
        cfg.set_alignment_horizontal(Entity::Row(row), align);
    }
}

fn shift_colors_down(cfg: &mut ColoredConfig, row: usize, count_rows: usize, count_columns: usize) {
    for row in row..count_rows {
        for col in 0..count_columns {
            let pos = (row + 1, col);
            let posn = (row, col).into();
            let color = cfg.get_colors().get_color(pos).cloned();
            if let Some(color) = color {
                cfg.set_color(posn, color);
            }
        }
    }
}

fn shift_lines_up(cfg: &mut ColoredConfig, count_rows: usize, lines: &[usize]) {
    for &i in lines {
        let line = cfg.get_horizontal_line(i).cloned();
        if let Some(line) = line {
            cfg.insert_horizontal_line(i - 1, line);
            cfg.remove_horizontal_line(i, count_rows);
        }
    }
}

fn theme_copy_horizontal_line(theme: &mut tabled::settings::Theme, from: usize, to: usize) {
    if let Some(line) = theme.get_horizontal_line(from) {
        theme.insert_horizontal_line(to, *line);
    }
}

#[allow(clippy::type_complexity)]
fn strip_color_from_row(row: usize) -> ModifyList<Row, FormatContent<fn(&str) -> String>> {
    fn foo(s: &str) -> String {
        strip_ansi_unlikely(s).into_owned()
    }

    Modify::new(Rows::single(row)).with(Format::content(foo))
}
