// TODO: Stop building `tabled -e` when it's clear we are out of terminal
// TODO: Stop building `tabled` when it's clear we are out of terminal
// NOTE: TODO the above we could expose something like [`WidthCtrl`] in which case we could also laverage the width list build right away.
//       currently it seems like we do recacalculate it for `table -e`?

use std::cmp::{max, min};

use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::{TableIndent, TrimStrategy};

use tabled::{
    Table,
    builder::Builder,
    grid::{
        ansi::ANSIBuf,
        colors::Colors,
        config::{
            AlignmentHorizontal, ColoredConfig, Entity, EntityMap, Indent, Position, Sides,
            SpannedConfig,
        },
        dimension::{CompleteDimensionVecRecords, SpannedGridDimension},
        records::{
            IntoRecords, IterRecords, Records,
            vec_records::{Cell, Text, VecRecords},
        },
    },
    settings::{
        Alignment, CellOption, Color, Padding, TableOption, Width,
        formatting::AlignmentStrategy,
        object::{Columns, Rows},
        themes::ColumnNames,
        width::Truncate,
    },
};

use crate::{convert_style, is_color_empty, table_theme::TableTheme};

const EMPTY_COLUMN_TEXT: &str = "...";
const EMPTY_COLUMN_TEXT_WIDTH: usize = 3;

pub type NuRecords = VecRecords<NuRecordsValue>;
pub type NuRecordsValue = Text<String>;

/// NuTable is a table rendering implementation.
#[derive(Debug, Clone)]
pub struct NuTable {
    data: Vec<Vec<NuRecordsValue>>,
    widths: Vec<usize>,
    count_rows: usize,
    count_cols: usize,
    styles: Styles,
    config: TableConfig,
}

impl NuTable {
    /// Creates an empty [`NuTable`] instance.
    pub fn new(count_rows: usize, count_cols: usize) -> Self {
        Self {
            data: vec![vec![Text::default(); count_cols]; count_rows],
            widths: vec![2; count_cols],
            count_rows,
            count_cols,
            styles: Styles {
                cfg: ColoredConfig::default(),
                alignments: CellConfiguration {
                    data: AlignmentHorizontal::Left,
                    index: AlignmentHorizontal::Right,
                    header: AlignmentHorizontal::Center,
                },
                colors: CellConfiguration::default(),
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
        self.count_rows
    }

    /// Return amount of columns.
    pub fn count_columns(&self) -> usize {
        self.count_cols
    }

    pub fn insert(&mut self, pos: Position, text: String) {
        let text = Text::new(text);
        self.widths[pos.1] = max(
            self.widths[pos.1],
            text.width() + indent_sum(self.config.indent),
        );
        self.data[pos.0][pos.1] = text;
    }

    pub fn set_row(&mut self, index: usize, row: Vec<NuRecordsValue>) {
        assert_eq!(self.data[index].len(), row.len());

        for (i, text) in row.iter().enumerate() {
            self.widths[i] = max(
                self.widths[i],
                text.width() + indent_sum(self.config.indent),
            );
        }

        self.data[index] = row;
    }

    pub fn pop_column(&mut self, count: usize) {
        self.count_cols -= count;
        self.widths.truncate(self.count_cols);
        for row in &mut self.data[..] {
            row.truncate(self.count_cols);
        }

        // set to default styles of the popped columns
        for i in 0..count {
            let col = self.count_cols + i;
            for row in 0..self.count_rows {
                self.styles
                    .cfg
                    .set_alignment_horizontal(Entity::Cell(row, col), self.styles.alignments.data);
                self.styles
                    .cfg
                    .set_color(Entity::Cell(row, col), ANSIBuf::default());
            }
        }
    }

    pub fn push_column(&mut self, text: String) {
        let value = Text::new(text);

        self.widths
            .push(value.width() + indent_sum(self.config.indent));

        for row in &mut self.data[..] {
            row.push(value.clone());
        }

        self.count_cols += 1;
    }

    pub fn insert_style(&mut self, pos: Position, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.cfg.set_color(pos.into(), style.into());
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.styles.alignments.data {
            self.styles
                .cfg
                .set_alignment_horizontal(pos.into(), alignment);
        }
    }

    pub fn set_header_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.colors.header = style;
        }

        self.styles.alignments.header = convert_alignment(style.alignment);
    }

    pub fn set_index_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            let style = convert_style(style);
            self.styles.colors.index = style;
        }

        self.styles.alignments.index = convert_alignment(style.alignment);
    }

    pub fn set_data_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style {
            if !style.is_plain() {
                let style = convert_style(style);
                self.styles.cfg.set_color(Entity::Global, style.into());
            }
        }

        let alignment = convert_alignment(style.alignment);
        self.styles
            .cfg
            .set_alignment_horizontal(Entity::Global, alignment);
        self.styles.alignments.data = alignment;
    }

    // NOTE: Crusial to be called before data changes (todo fix interface)
    pub fn set_indent(&mut self, indent: TableIndent) {
        self.config.indent = indent;

        let pad = indent_sum(indent);
        for w in &mut self.widths {
            *w = pad;
        }
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

    pub fn clear_border_color(&mut self) {
        self.config.border_color = None;
    }

    // NOTE: BE CAREFUL TO KEEP WIDTH UNCHANGED
    // TODO: fix interface
    pub fn get_records_mut(&mut self) -> &mut [Vec<NuRecordsValue>] {
        &mut self.data
    }

    pub fn clear_all_colors(&mut self) {
        self.clear_border_color();
        self.styles.cfg.set_colors(EntityMap::default());
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, termwidth: usize) -> Option<String> {
        build_table(self, termwidth)
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw_unchecked(self, termwidth: usize) -> Option<String> {
        build_table_unchecked(self, termwidth)
    }

    /// Return a total table width.
    pub fn total_width(&self) -> usize {
        let config = create_config(&self.config.theme, false, None);
        get_total_width2(&self.widths, &config)
    }
}

impl From<Vec<Vec<Text<String>>>> for NuTable {
    fn from(value: Vec<Vec<Text<String>>>) -> Self {
        let count_rows = value.len();
        let count_cols = if value.is_empty() { 0 } else { value[0].len() };

        let mut t = Self::new(count_rows, count_cols);
        t.data = value;
        table_recalculate_widths(&mut t);

        t
    }
}

fn table_recalculate_widths(t: &mut NuTable) {
    let pad = indent_sum(t.config.indent);
    let records = IterRecords::new(&t.data, t.count_cols, Some(t.count_rows));
    let widths = build_width(records, pad);
    t.widths = widths;
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Hash)]
struct CellConfiguration<Value> {
    index: Value,
    header: Value,
    data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Styles {
    cfg: ColoredConfig,
    colors: CellConfiguration<Color>,
    alignments: CellConfiguration<AlignmentHorizontal>,
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

fn build_table_unchecked(mut t: NuTable, termwidth: usize) -> Option<String> {
    if t.count_columns() == 0 || t.count_rows() == 0 {
        return Some(String::new());
    }

    let widths = std::mem::take(&mut t.widths);
    let config = create_config(&t.config.theme, false, None);
    let totalwidth = get_total_width2(&t.widths, &config);
    let widths = WidthEstimation::new(widths.clone(), widths, totalwidth, false, false);

    let head = remove_header_if(&mut t);
    table_insert_footer_if(&mut t);

    draw_table(t, widths, head, termwidth)
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

fn table_truncate(t: &mut NuTable, termwidth: usize) -> Option<WidthEstimation> {
    let widths = maybe_truncate_columns(&mut t.data, t.widths.clone(), &t.config, termwidth);
    if widths.needed.is_empty() {
        return None;
    }

    // reset style for last column which is a trail one
    if widths.trail {
        let col = widths.needed.len() - 1;
        for row in 0..t.count_rows {
            t.styles
                .cfg
                .set_alignment_horizontal(Entity::Cell(row, col), t.styles.alignments.data);
            t.styles
                .cfg
                .set_color(Entity::Cell(row, col), ANSIBuf::default());
        }
    }

    Some(widths)
}

fn remove_header(t: &mut NuTable) -> HeadInfo {
    // move settings by one row down
    for row in 1..t.data.len() {
        for col in 0..t.count_cols {
            let alignment = *t
                .styles
                .cfg
                .get_alignment_horizontal(Entity::Cell(row, col));
            if alignment != t.styles.alignments.data {
                t.styles
                    .cfg
                    .set_alignment_horizontal(Entity::Cell(row - 1, col), alignment);
            }

            // TODO: use get_color from upstream (when released)
            let color = t.styles.cfg.get_colors().get_color((row, col)).cloned();
            if let Some(color) = color {
                t.styles.cfg.set_color(Entity::Cell(row - 1, col), color);
            }
        }
    }

    let head = t
        .data
        .remove(0)
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // WE NEED TO RELCULATE WIDTH.
    // TODO: cause we have configuration beforehand we can just not calculate it in?
    table_recalculate_widths(t);

    let alignment = t.styles.alignments.header;
    let color = get_color_if_exists(&t.styles.colors.header);

    t.styles.alignments.header = AlignmentHorizontal::Center;
    t.styles.colors.header = Color::empty();

    HeadInfo::new(head, alignment, color)
}

fn draw_table(
    t: NuTable,
    width: WidthEstimation,
    head: Option<HeadInfo>,
    termwidth: usize,
) -> Option<String> {
    let mut structure = t.config.structure;
    structure.with_footer = structure.with_footer && head.is_none();
    let sep_color = t.config.border_color;

    let data = t.data;
    let mut table = Builder::from_vec(data).build();

    set_styles(&mut table, t.styles, &structure);
    set_indent(&mut table, t.config.indent);
    load_theme(&mut table, &t.config.theme, &structure, sep_color);
    truncate_table(&mut table, &t.config, width, termwidth);
    table_set_border_header(&mut table, head, &t.config);

    table_to_string(table, termwidth)
}

fn set_styles(table: &mut Table, styles: Styles, structure: &TableStructure) {
    table.with(styles.cfg);
    align_table(table, styles.alignments, structure);
    colorize_table(table, styles.colors, structure);
}

fn table_set_border_header(table: &mut Table, head: Option<HeadInfo>, cfg: &TableConfig) {
    let head = match head {
        Some(head) => head,
        None => return,
    };

    let theme = &cfg.theme;
    let with_footer = cfg.structure.with_footer;
    let pad = cfg.indent.left + cfg.indent.right;

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
        table.with(SetLineHeaders::new(head.clone(), last_row, pad));
    }

    table.with(SetLineHeaders::new(head, 0, pad));
}

fn truncate_table(table: &mut Table, cfg: &TableConfig, width: WidthEstimation, termwidth: usize) {
    let trim = cfg.trim.clone();
    let pad = cfg.indent.left + cfg.indent.right;
    let ctrl = WidthCtrl::new(termwidth, width, trim, cfg.expand, pad);
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
    width: WidthEstimation,
    trim_strategy: TrimStrategy,
    max_width: usize,
    expand: bool,
    pad: usize,
}

impl WidthCtrl {
    fn new(
        max_width: usize,
        width: WidthEstimation,
        trim_strategy: TrimStrategy,
        expand: bool,
        pad: usize,
    ) -> Self {
        Self {
            width,
            trim_strategy,
            max_width,
            expand,
            pad,
        }
    }
}

#[derive(Debug, Clone)]
struct WidthEstimation {
    original: Vec<usize>,
    needed: Vec<usize>,
    #[allow(dead_code)]
    total: usize,
    truncate: bool,
    trail: bool,
}

impl WidthEstimation {
    fn new(
        original: Vec<usize>,
        needed: Vec<usize>,
        total: usize,
        truncate: bool,
        trail: bool,
    ) -> Self {
        Self {
            original,
            needed,
            total,
            truncate,
            trail,
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
        if self.width.truncate {
            width_ctrl_truncate(self, recs, cfg, dims);
            return;
        }

        if self.expand {
            width_ctrl_expand(self, recs, cfg, dims);
            return;
        }

        // NOTE: just an optimization; to not recalculate it internally
        dims.set_widths(self.width.needed);
    }
}

fn width_ctrl_expand(
    ctrl: WidthCtrl,
    recs: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords,
) {
    let opt = Width::increase(ctrl.max_width);
    TableOption::<VecRecords<_>, _, _>::change(opt, recs, cfg, dims);
}

fn width_ctrl_truncate(
    ctrl: WidthCtrl,
    recs: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords,
) {
    for (col, (&width, width_original)) in ctrl
        .width
        .needed
        .iter()
        .zip(ctrl.width.original)
        .enumerate()
    {
        if width == width_original {
            continue;
        }

        let width = width - ctrl.pad;

        match &ctrl.trim_strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let wrap = Width::wrap(width).keep_words(*try_to_keep_words);

                CellOption::<NuRecords, _>::change(wrap, recs, cfg, Entity::Column(col));
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(width);
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                CellOption::<NuRecords, _>::change(truncate, recs, cfg, Entity::Column(col));
            }
        }
    }

    dims.set_widths(ctrl.width.needed);
}

fn align_table(
    table: &mut Table,
    alignments: CellConfiguration<AlignmentHorizontal>,
    structure: &TableStructure,
) {
    table.with(AlignmentStrategy::PerLine);

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

fn colorize_table(table: &mut Table, styles: CellConfiguration<Color>, structure: &TableStructure) {
    if structure.with_index && !is_color_empty(&styles.index) {
        table.modify(Columns::first(), styles.index);
    }

    if structure.with_header && !is_color_empty(&styles.header) {
        table.modify(Rows::first(), styles.header.clone());
    }

    if structure.with_header && structure.with_footer && !is_color_empty(&styles.header) {
        table.modify(Rows::last(), styles.header);
    }
}

fn load_theme(
    table: &mut Table,
    theme: &TableTheme,
    structure: &TableStructure,
    sep_color: Option<Style>,
) {
    let with_header = table.count_rows() > 1 && structure.with_header;
    let with_footer = with_header && structure.with_footer;
    let mut theme = theme.as_base().clone();

    if !with_header {
        let borders = *theme.get_borders();
        theme.remove_horizontal_lines();
        theme.set_borders(borders);
    } else if with_footer {
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
    data: &mut Vec<Vec<NuRecordsValue>>,
    widths: Vec<usize>,
    cfg: &TableConfig,
    termwidth: usize,
) -> WidthEstimation {
    const TERMWIDTH_THRESHOLD: usize = 120;

    let pad = cfg.indent.left + cfg.indent.right;
    let preserve_content = termwidth > TERMWIDTH_THRESHOLD;

    if preserve_content {
        truncate_columns_by_columns(data, widths, &cfg.theme, pad, termwidth)
    } else {
        truncate_columns_by_content(data, widths, &cfg.theme, pad, termwidth)
    }
}

// VERSION where we are showing AS LITTLE COLUMNS AS POSSIBLE but WITH AS MUCH CONTENT AS POSSIBLE.
fn truncate_columns_by_content(
    data: &mut Vec<Vec<NuRecordsValue>>,
    widths: Vec<usize>,
    theme: &TableTheme,
    pad: usize,
    termwidth: usize,
) -> WidthEstimation {
    const MIN_ACCEPTABLE_WIDTH: usize = 5;
    const TRAILING_COLUMN_WIDTH: usize = EMPTY_COLUMN_TEXT_WIDTH;

    let trailing_column_width = TRAILING_COLUMN_WIDTH + pad;
    let min_column_width = MIN_ACCEPTABLE_WIDTH + pad;

    let count_columns = data[0].len();

    let config = create_config(theme, false, None);
    let widths_original = widths;
    let mut widths = vec![];

    let borders = config.get_borders();
    let vertical = borders.has_vertical() as usize;

    let mut width = borders.has_left() as usize + borders.has_right() as usize;
    let mut truncate_pos = 0;

    for (i, &column_width) in widths_original.iter().enumerate() {
        let mut next_move = column_width;
        if i > 0 {
            next_move += vertical;
        }

        if width + next_move > termwidth {
            break;
        }

        widths.push(column_width);
        width += next_move;
        truncate_pos += 1;
    }

    if truncate_pos == count_columns {
        return WidthEstimation::new(widths_original, widths, width, false, false);
    }

    if truncate_pos == 0 {
        if termwidth > width {
            let available = termwidth - width;
            if available >= min_column_width + vertical + trailing_column_width {
                truncate_rows(data, 1);

                let first_col_width = available - (vertical + trailing_column_width);
                widths.push(first_col_width);
                width += first_col_width;

                push_empty_column(data);
                widths.push(trailing_column_width);
                width += trailing_column_width + vertical;

                return WidthEstimation::new(widths_original, widths, width, true, true);
            }
        }

        return WidthEstimation::new(widths_original, widths, width, false, false);
    }

    let available = termwidth - width;

    let is_last_column = truncate_pos + 1 == count_columns;
    let can_fit_last_column = available >= min_column_width + vertical;
    if is_last_column && can_fit_last_column {
        let w = available - vertical;
        widths.push(w);
        width += w + vertical;

        return WidthEstimation::new(widths_original, widths, width, true, false);
    }

    // special case where the last column is smaller then a trailing column
    let is_almost_last_column = truncate_pos + 2 == count_columns;
    if is_almost_last_column {
        let next_column_width = widths_original[truncate_pos + 1];
        let has_space_for_two_columns =
            available >= min_column_width + vertical + next_column_width + vertical;

        if !is_last_column && has_space_for_two_columns {
            let rest = available - vertical - next_column_width - vertical;
            widths.push(rest);
            width += rest + vertical;

            widths.push(next_column_width);
            width += next_column_width + vertical;

            return WidthEstimation::new(widths_original, widths, width, true, false);
        }
    }

    let has_space_for_two_columns =
        available >= min_column_width + vertical + trailing_column_width + vertical;
    if !is_last_column && has_space_for_two_columns {
        truncate_rows(data, truncate_pos + 1);

        let rest = available - vertical - trailing_column_width - vertical;
        widths.push(rest);
        width += rest + vertical;

        push_empty_column(data);
        widths.push(trailing_column_width);
        width += trailing_column_width + vertical;

        return WidthEstimation::new(widths_original, widths, width, true, true);
    }

    if available >= trailing_column_width + vertical {
        truncate_rows(data, truncate_pos);

        push_empty_column(data);
        widths.push(trailing_column_width);
        width += trailing_column_width + vertical;

        return WidthEstimation::new(widths_original, widths, width, false, true);
    }

    let last_width = widths.last().cloned().expect("ok");
    let can_truncate_last = last_width > min_column_width;

    if can_truncate_last {
        let rest = last_width - min_column_width;
        let maybe_available = available + rest;

        if maybe_available >= trailing_column_width + vertical {
            truncate_rows(data, truncate_pos);

            let left = maybe_available - trailing_column_width - vertical;
            let new_last_width = min_column_width + left;

            widths[truncate_pos - 1] = new_last_width;
            width -= last_width;
            width += new_last_width;

            push_empty_column(data);
            widths.push(trailing_column_width);
            width += trailing_column_width + vertical;

            return WidthEstimation::new(widths_original, widths, width, true, true);
        }
    }

    truncate_rows(data, truncate_pos - 1);
    let w = widths.pop().expect("ok");
    width -= w;

    push_empty_column(data);
    widths.push(trailing_column_width);
    width += trailing_column_width;

    if widths.len() == 1 {
        // nothing to show anyhow
        return WidthEstimation::new(widths_original, vec![], width, false, true);
    }

    WidthEstimation::new(widths_original, widths, width, false, true)
}

// VERSION where we are showing AS MANY COLUMNS AS POSSIBLE but as a side affect they MIGHT CONTAIN AS LITTLE CONTENT AS POSSIBLE
//
// TODO: Currently there's no prioritization of anything meaning all columns are equal
//       But I'd suggest to try to give a little more space for left most columns
//
//       So for example for instead of columns [10, 10, 10]
//       We would get [15, 10, 5]
//
//       Point being of the column needs more space we do can give it a little more based on it's distance from the start.
//       Percentage wise.
fn truncate_columns_by_columns(
    data: &mut Vec<Vec<NuRecordsValue>>,
    widths: Vec<usize>,
    theme: &TableTheme,
    pad: usize,
    termwidth: usize,
) -> WidthEstimation {
    const MIN_ACCEPTABLE_WIDTH: usize = 10;
    const TRAILING_COLUMN_WIDTH: usize = EMPTY_COLUMN_TEXT_WIDTH;

    let trailing_column_width = TRAILING_COLUMN_WIDTH + pad;
    let min_column_width = MIN_ACCEPTABLE_WIDTH + pad;

    let count_columns = data[0].len();

    let config = create_config(theme, false, None);
    let widths_original = widths;
    let mut widths = vec![];

    let borders = config.get_borders();
    let vertical = borders.has_vertical() as usize;

    let mut width = borders.has_left() as usize + borders.has_right() as usize;
    let mut truncate_pos = 0;

    for (i, &width_orig) in widths_original.iter().enumerate() {
        let use_width = min(min_column_width, width_orig);
        let mut next_move = use_width;
        if i > 0 {
            next_move += vertical;
        }

        if width + next_move > termwidth {
            break;
        }

        widths.push(use_width);
        width += next_move;
        truncate_pos += 1;
    }

    if truncate_pos == 0 {
        return WidthEstimation::new(widths_original, widths, width, false, false);
    }

    let mut available = termwidth - width;

    if available > 0 {
        for i in 0..truncate_pos {
            let used_width = widths[i];
            let col_width = widths_original[i];
            if used_width < col_width {
                let need = col_width - used_width;
                let take = min(available, need);
                available -= take;

                widths[i] += take;
                width += take;

                if available == 0 {
                    break;
                }
            }
        }
    }

    if truncate_pos == count_columns {
        return WidthEstimation::new(widths_original, widths, width, true, false);
    }

    if available >= trailing_column_width + vertical {
        truncate_rows(data, truncate_pos);

        push_empty_column(data);
        widths.push(trailing_column_width);
        width += trailing_column_width + vertical;

        return WidthEstimation::new(widths_original, widths, width, true, true);
    }

    truncate_rows(data, truncate_pos - 1);
    let w = widths.pop().expect("ok");
    width -= w;

    push_empty_column(data);
    widths.push(trailing_column_width);
    width += trailing_column_width;

    WidthEstimation::new(widths_original, widths, width, true, true)
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

fn push_empty_column(data: &mut Vec<Vec<NuRecordsValue>>) {
    let empty_cell = Text::new(String::from(EMPTY_COLUMN_TEXT));
    for row in data {
        row.push(empty_cell.clone());
    }
}

fn duplicate_row(data: &mut Vec<Vec<NuRecordsValue>>, row: usize) {
    let duplicate = data[row].clone();
    data.push(duplicate);
}

fn truncate_rows(data: &mut Vec<Vec<NuRecordsValue>>, count: usize) {
    for row in data {
        row.truncate(count);
    }
}

fn convert_alignment(alignment: nu_color_config::Alignment) -> AlignmentHorizontal {
    match alignment {
        nu_color_config::Alignment::Center => AlignmentHorizontal::Center,
        nu_color_config::Alignment::Left => AlignmentHorizontal::Left,
        nu_color_config::Alignment::Right => AlignmentHorizontal::Right,
    }
}

fn build_width<R>(records: R, pad: usize) -> Vec<usize>
where
    R: Records,
    <R::Iter as IntoRecords>::Cell: AsRef<str>,
{
    // TODO: Expose not spaned version (could be optimized).
    let mut cfg = SpannedConfig::default();
    let padding = Sides {
        left: Indent::spaced(pad),
        ..Default::default()
    };

    cfg.set_padding(Entity::Global, padding);

    // TODO: Use peekable width
    SpannedGridDimension::width(records, &cfg)
}

// It's laverages a use of guuaranted cached widths before hand
// to speed up things a bit.
struct SetLineHeaders {
    line: usize,
    pad: usize,
    head: HeadInfo,
}

impl SetLineHeaders {
    fn new(head: HeadInfo, line: usize, pad: usize) -> Self {
        Self { line, head, pad }
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
            .map(|(s, width)| Truncate::truncate(&s, width - self.pad).into_owned())
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

// todo: create a method
#[derive(Debug, Default)]
struct GetDims(Vec<usize>);

impl<R, C> TableOption<R, C, CompleteDimensionVecRecords<'_>> for &mut GetDims {
    fn change(self, _: &mut R, _: &mut C, dims: &mut CompleteDimensionVecRecords<'_>) {
        self.0 = dims.get_widths().expect("expected to get it").to_vec();
    }

    fn hint_change(&self) -> Option<Entity> {
        None
    }
}

pub fn get_color_if_exists(c: &Color) -> Option<Color> {
    if !is_color_empty(c) {
        Some(c.clone())
    } else {
        None
    }
}
