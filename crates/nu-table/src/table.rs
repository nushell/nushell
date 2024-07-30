use crate::{convert_style, table_theme::TableTheme};
use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::TrimStrategy;
use nu_utils::strip_ansi_unlikely;
use std::{cmp::min, collections::HashMap};
use tabled::{
    builder::Builder,
    grid::{
        color::AnsiColor,
        colors::Colors,
        config::{AlignmentHorizontal, ColoredConfig, Entity, EntityMap, Position},
        dimension::CompleteDimensionVecRecords,
        records::{
            vec_records::{Cell, CellInfo, VecRecords},
            ExactRecords, PeekableRecords, Records, Resizable,
        },
    },
    settings::{
        formatting::AlignmentStrategy,
        object::{Columns, Segment},
        peaker::Peaker,
        themes::ColumnNames,
        width::Truncate,
        Color, Modify, Padding, Settings, TableOption, Width,
    },
    Table,
};

/// NuTable is a table rendering implementation.
#[derive(Debug, Clone)]
pub struct NuTable {
    data: NuRecords,
    styles: Styles,
    alignments: Alignments,
    indent: (usize, usize),
}

pub type NuRecords = VecRecords<NuTableCell>;
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
    /// Creates an empty [`NuTable`] instance.
    pub fn new(count_rows: usize, count_columns: usize) -> Self {
        Self {
            data: VecRecords::new(vec![vec![CellInfo::default(); count_columns]; count_rows]),
            styles: Styles::default(),
            indent: (1, 1),
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

    pub fn set_indent(&mut self, left: usize, right: usize) {
        self.indent = (left, right);
    }

    pub fn get_records_mut(&mut self) -> &mut NuRecords {
        &mut self.data
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, config: NuTableConfig, termwidth: usize) -> Option<String> {
        build_table(
            self.data,
            config,
            self.alignments,
            self.styles,
            termwidth,
            self.indent,
        )
    }

    /// Return a total table width.
    pub fn total_width(&self, config: &NuTableConfig) -> usize {
        let config = get_config(&config.theme, false, None);
        let widths = build_width(&self.data, self.indent.0 + self.indent.1);
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
    mut data: NuRecords,
    cfg: NuTableConfig,
    alignments: Alignments,
    styles: Styles,
    termwidth: usize,
    indent: (usize, usize),
) -> Option<String> {
    if data.count_columns() == 0 || data.count_rows() == 0 {
        return Some(String::new());
    }

    let pad = indent.0 + indent.1;
    let widths = maybe_truncate_columns(&mut data, &cfg, termwidth, pad);
    if widths.is_empty() {
        return None;
    }

    if cfg.with_header && cfg.with_footer {
        duplicate_row(&mut data, 0);
    }

    draw_table(data, alignments, styles, widths, cfg, termwidth, indent)
}

fn draw_table(
    data: NuRecords,
    alignments: Alignments,
    styles: Styles,
    widths: Vec<usize>,
    cfg: NuTableConfig,
    termwidth: usize,
    indent: (usize, usize),
) -> Option<String> {
    let with_index = cfg.with_index;
    let with_header = cfg.with_header && data.count_rows() > 1;
    let with_footer = with_header && cfg.with_footer;
    let sep_color = cfg.split_color;
    let border_header = cfg.header_on_border;

    let data: Vec<Vec<_>> = data.into();
    let mut table = Builder::from(data).build();

    set_indent(&mut table, indent.0, indent.1);
    load_theme(&mut table, &cfg.theme, with_footer, with_header, sep_color);
    align_table(&mut table, alignments, with_index, with_header, with_footer);
    colorize_table(&mut table, styles, with_index, with_header, with_footer);

    let pad = indent.0 + indent.1;
    let width_ctrl = TableWidthCtrl::new(widths, cfg, termwidth, pad);

    if with_header && border_header {
        set_border_head(&mut table, with_footer, width_ctrl);
    } else {
        table.with(width_ctrl);
    }

    table_to_string(table, termwidth)
}

fn set_indent(table: &mut Table, left: usize, right: usize) {
    table.with(Padding::new(left, right, 0, 0));
}

fn set_border_head(table: &mut Table, with_footer: bool, wctrl: TableWidthCtrl) {
    if with_footer {
        let count_rows = table.count_rows();
        let last_row_index = count_rows - 1;

        // note: funnily last and row must be equal at this point but we do not rely on it just in case.

        let mut first_row = GetRow(0, Vec::new());
        let mut head_settings = GetRowSettings(0, AlignmentHorizontal::Left, None);
        let mut last_row = GetRow(last_row_index, Vec::new());

        table.with(&mut first_row);
        table.with(&mut head_settings);
        table.with(&mut last_row);

        table.with(
            Settings::default()
                .with(wctrl)
                .with(StripColorFromRow(0))
                .with(StripColorFromRow(count_rows - 1))
                .with(MoveRowNext::new(0, 0))
                .with(MoveRowPrev::new(last_row_index - 1, last_row_index))
                .with(SetLineHeaders::new(
                    0,
                    first_row.1,
                    head_settings.1,
                    head_settings.2.clone(),
                ))
                .with(SetLineHeaders::new(
                    last_row_index - 1,
                    last_row.1,
                    head_settings.1,
                    head_settings.2,
                )),
        );
    } else {
        let mut row = GetRow(0, Vec::new());
        let mut row_opts = GetRowSettings(0, AlignmentHorizontal::Left, None);

        table.with(&mut row);
        table.with(&mut row_opts);

        table.with(
            Settings::default()
                .with(wctrl)
                .with(StripColorFromRow(0))
                .with(MoveRowNext::new(0, 0))
                .with(SetLineHeaders::new(0, row.1, row_opts.1, row_opts.2)),
        );
    }
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

struct TableWidthCtrl {
    width: Vec<usize>,
    cfg: NuTableConfig,
    width_max: usize,
    pad: usize,
}

impl TableWidthCtrl {
    fn new(width: Vec<usize>, cfg: NuTableConfig, max: usize, pad: usize) -> Self {
        Self {
            width,
            cfg,
            width_max: max,
            pad,
        }
    }
}

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for TableWidthCtrl {
    fn change(
        self,
        rec: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dim: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let total_width = get_total_width2(&self.width, cfg);

        if total_width > self.width_max {
            let has_header = self.cfg.with_header && rec.count_rows() > 1;
            let trim_as_head = has_header && self.cfg.header_on_border;

            TableTrim::new(
                self.width,
                self.width_max,
                self.cfg.trim,
                trim_as_head,
                self.pad,
            )
            .change(rec, cfg, dim);
        } else if self.cfg.expand && self.width_max > total_width {
            Settings::new(SetDimensions(self.width), Width::increase(self.width_max))
                .change(rec, cfg, dim)
        } else {
            SetDimensions(self.width).change(rec, cfg, dim);
        }
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

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for TableTrim {
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
                let mut wrap = Width::wrap(self.width_max).priority::<PriorityMax>();
                if try_to_keep_words {
                    wrap = wrap.keep_words();
                }

                Settings::new(SetDimensions(self.width), wrap).change(recs, cfg, dims);
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(self.width_max).priority::<PriorityMax>();
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                Settings::new(SetDimensions(self.width), truncate).change(recs, cfg, dims);
            }
        }
    }
}

fn trim_as_header(
    recs: &mut VecRecords<CellInfo<String>>,
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
        .map(CellInfo::width)
        .map(|v| v + trim.pad)
        .collect::<Vec<_>>();
    let min_width_use = get_total_width2(&headers_widths, cfg);
    let mut free_width = trim.width_max.saturating_sub(min_width_use);

    // even though it's safe to trim columns by header there might be left unused space
    // so we do use it if possible prioritizing left columns

    for (i, head_width) in headers_widths.into_iter().enumerate() {
        let head_width = head_width - trim.pad;
        let column_width = trim.width[i] - trim.pad; // safe to assume width is bigger then paddding

        let mut use_width = head_width;
        if free_width > 0 {
            // it's safe to assume that column_width is always bigger or equal to head_width
            debug_assert!(column_width >= head_width);

            let additional_width = min(free_width, column_width - head_width);
            free_width -= additional_width;
            use_width += additional_width;
        }

        match &trim.strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                let mut wrap = Width::wrap(use_width);
                if *try_to_keep_words {
                    wrap = wrap.keep_words();
                }

                Modify::new(Columns::single(i))
                    .with(wrap)
                    .change(recs, cfg, dims);
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(use_width);
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                Modify::new(Columns::single(i))
                    .with(truncate)
                    .change(recs, cfg, dims);
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

fn maybe_truncate_columns(
    data: &mut NuRecords,
    cfg: &NuTableConfig,
    termwidth: usize,
    pad: usize,
) -> Vec<usize> {
    const TERMWIDTH_THRESHOLD: usize = 120;

    let preserve_content = termwidth > TERMWIDTH_THRESHOLD;
    let has_header = cfg.with_header && data.count_rows() > 1;
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

    let config = get_config(theme, false, None);
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

    let config = get_config(theme, false, None);
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

    let config = get_config(theme, false, None);
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

fn push_empty_column(data: &mut NuRecords) {
    let records = std::mem::take(data);
    let mut inner: Vec<Vec<_>> = records.into();

    let empty_cell = CellInfo::new(String::from("..."));
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

struct SetAlignment(AlignmentHorizontal, Entity);

impl<R, D> TableOption<R, D, ColoredConfig> for SetAlignment {
    fn change(self, _: &mut R, cfg: &mut ColoredConfig, _: &mut D) {
        cfg.set_alignment_horizontal(self.1, self.0);
    }
}

struct SetDimensions(Vec<usize>);

impl<R> TableOption<R, CompleteDimensionVecRecords<'_>, ColoredConfig> for SetDimensions {
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

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for &mut GetRow {
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

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig>
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

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for SetLineHeaders {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        dims: &mut CompleteDimensionVecRecords<'_>,
    ) {
        let mut columns = self.columns;
        match dims.get_widths() {
            Some(widths) => {
                columns = columns
                    .into_iter()
                    .zip(widths.iter().map(|w| w.checked_sub(2).unwrap_or(*w))) // exclude padding; which is generally 2
                    .map(|(s, width)| Truncate::truncate_text(&s, width).into_owned())
                    .collect();
            }
            None => {
                // we don't have widths cached; which means that NO width adjustments were done
                // which means we are OK to leave columns as they are.
                //
                // but we actually always have to have widths at this point
            }
        };

        set_column_names(
            recs,
            cfg,
            dims,
            columns,
            self.line,
            self.alignment,
            self.color,
        )
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

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for MoveRowNext {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        row_shift_next(recs, cfg, self.row, self.line);
    }
}

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for MoveRowPrev {
    fn change(
        self,
        recs: &mut NuRecords,
        cfg: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        row_shift_prev(recs, cfg, self.row, self.line);
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

    if !has_line {
        let _ = remove_row(recs, row);
        let count_rows = recs.count_rows();
        shift_alignments_down(cfg, row, count_rows, count_columns);
        shift_colors_down(cfg, row, count_rows, count_columns);
        shift_lines_up(cfg, count_rows, &[line + 1]);
        shift_lines_up(cfg, count_rows, &[count_rows]);
        return;
    }

    let _ = remove_row(recs, row);
    let count_rows = recs.count_rows();
    shift_alignments_down(cfg, row, count_rows, count_columns);
    shift_colors_down(cfg, row, count_rows, count_columns);
    remove_lines(cfg, count_rows, &[line + 1]);
    shift_lines_up(cfg, count_rows, &[count_rows]);
}

fn row_shift_prev(recs: &mut NuRecords, cfg: &mut ColoredConfig, row: usize, line: usize) {
    let count_rows = recs.count_rows();
    let count_columns = recs.count_columns();
    let has_line = cfg.has_horizontal(line, count_rows);
    let has_prev_line = cfg.has_horizontal(line - 1, count_rows);
    if !has_line && !has_prev_line {
        return;
    }

    if !has_line {
        let _ = remove_row(recs, row);
        // shift_lines_down(table, &[line - 1]);
        return;
    }

    let _ = remove_row(recs, row);
    let count_rows = count_rows - 1;
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

fn set_column_names(
    records: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimensionVecRecords<'_>,
    head: Vec<String>,
    line: usize,
    align: AlignmentHorizontal,
    color: Option<Color>,
) {
    let mut names = ColumnNames::new(head).set_line(line).set_alignment(align);
    if let Some(color) = color {
        names = names.set_color(color);
    }

    ColumnNames::change(names, records, cfg, dims)
}

fn remove_row(recs: &mut NuRecords, row: usize) -> Vec<String> {
    let count_columns = recs.count_columns();
    let columns = (0..count_columns)
        .map(|column| recs.get_text((row, column)).to_owned())
        .collect::<Vec<_>>();

    recs.remove_row(row);

    columns
}

struct StripColorFromRow(usize);

impl TableOption<NuRecords, CompleteDimensionVecRecords<'_>, ColoredConfig> for StripColorFromRow {
    fn change(
        self,
        recs: &mut NuRecords,
        _: &mut ColoredConfig,
        _: &mut CompleteDimensionVecRecords<'_>,
    ) {
        for cell in &mut recs[self.0] {
            *cell = CellInfo::new(strip_ansi_unlikely(cell.as_ref()).into_owned());
        }
    }
}
