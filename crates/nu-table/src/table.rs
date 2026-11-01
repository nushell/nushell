use crate::{convert_style, is_color_empty, string_width, table_theme::TableTheme};
use nu_ansi_term::Style;
use nu_color_config::TextStyle;
use nu_protocol::{TableIndent, TrimStrategy};
use std::cmp::{Reverse, max, min};
use std::sync::{Arc, LazyLock};
use tabled::{
    Table,
    builder::Builder,
    grid::{
        ansi::ANSIBuf,
        colors::Colors,
        config::{
            AlignmentHorizontal, Border, Borders, ColoredConfig, Entity, Indent, Position, Sides,
            SpannedConfig,
        },
        dimension::{CompleteDimension, PeekableGridDimension},
        records::{
            IterRecords, PeekableRecords,
            vec_records::{Cell, Text, VecRecords},
        },
        util::string::get_char_width,
    },
    settings::{
        Alignment, CellOption, Color, Padding, TableOption, Width,
        formatting::AlignmentStrategy,
        object::{Columns, Rows},
        themes::ColumnNames,
        width::Truncate,
    },
};

const EMPTY_COLUMN_TEXT: &str = "...";
const EMPTY_COLUMN_TEXT_WIDTH: usize = 3;
/// Narrowest content width worth wrapping into, not counting padding. The first
/// data column may still shrink to it when not even its floor fits; narrower
/// columns are cut instead of wrapped.
const MIN_COLUMN_CONTENT_WIDTH: usize = 4;
/// Content width a column keeps, when it is that wide, before columns are hidden.
const MIN_READABLE_WIDTH: usize = 10;

/// The theme of a new table, shared so that a table costs no theme of its own
/// until [`NuTable::set_theme`] gives it one.
static BASIC_THEME: LazyLock<Arc<TableTheme>> = LazyLock::new(|| Arc::new(TableTheme::basic()));

pub type NuRecords = VecRecords<NuRecordsValue>;
pub type NuRecordsValue = Text<String>;

/// NuTable is a table rendering implementation.
#[derive(Debug, Clone)]
pub struct NuTable {
    data: Vec<Vec<NuRecordsValue>>,
    widths: Vec<usize>,
    /// The narrowest width (padding included) each column may get to draw the
    /// nested tables it holds: room for their borders and first column.
    min_widths: Vec<usize>,
    /// The same, with room to draw the nested tables in that first column too.
    deep_min_widths: Vec<usize>,
    heights: Vec<usize>,
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
            min_widths: vec![0; count_cols],
            deep_min_widths: vec![0; count_cols],
            heights: vec![0; count_rows],
            count_rows,
            count_cols,
            styles: Styles {
                cell_colors: vec![],
                cell_alignments: vec![],
                alignments: CellConfiguration {
                    data: AlignmentHorizontal::Left,
                    index: AlignmentHorizontal::Right,
                    header: AlignmentHorizontal::Center,
                },
                colors: CellConfiguration::default(),
            },
            config: TableConfig {
                theme: Arc::clone(&BASIC_THEME),
                trim: TrimStrategy::truncate(None),
                structure: TableStructure::new(false, false, false),
                indent: TableIndent::new(1, 1),
                header_on_border: false,
                expand: false,
                border_color: None,
                width_priority_columns: vec![],
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

    pub fn insert(&mut self, pos: (usize, usize), text: String) {
        let text = Text::new(text);
        let pad = indent_sum(self.config.indent);
        let width = text.width() + pad;
        let height = text.count_lines();
        self.widths[pos.1] = max(self.widths[pos.1], width);
        self.heights[pos.0] = max(self.heights[pos.0], height);
        self.data[pos.0][pos.1] = text;
    }

    pub fn set_row(&mut self, index: usize, row: Vec<NuRecordsValue>) {
        assert_eq!(self.data[index].len(), row.len());

        for (i, text) in row.iter().enumerate() {
            let pad = indent_sum(self.config.indent);
            let width = text.width() + pad;
            let height = text.count_lines();

            self.widths[i] = max(self.widths[i], width);
            self.heights[index] = max(self.heights[index], height);
        }

        self.data[index] = row;
    }

    pub fn insert_style(&mut self, pos: (usize, usize), style: TextStyle) {
        let entity = Entity::Cell(pos.0, pos.1);
        if let Some(style) = style.color_style
            && !style.is_plain()
        {
            let style = convert_style(style);
            self.styles.cell_colors.push((entity, style.into()));
        }

        let alignment = convert_alignment(style.alignment);
        if alignment != self.styles.alignments.data {
            self.styles.cell_alignments.push((entity, alignment));
        }
    }

    pub fn set_header_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style
            && !style.is_plain()
        {
            let style = convert_style(style);
            self.styles.colors.header = style;
        }

        self.styles.alignments.header = convert_alignment(style.alignment);
    }

    pub fn set_index_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style
            && !style.is_plain()
        {
            let style = convert_style(style);
            self.styles.colors.index = style;
        }

        self.styles.alignments.index = convert_alignment(style.alignment);
    }

    pub fn set_data_style(&mut self, style: TextStyle) {
        if let Some(style) = style.color_style
            && !style.is_plain()
        {
            let style = convert_style(style);
            self.styles.cell_colors.push((Entity::Global, style.into()));
        }

        let alignment = convert_alignment(style.alignment);
        self.styles
            .cell_alignments
            .push((Entity::Global, alignment));
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

    /// Sets the theme. Tables drawn together can share one `Arc<TableTheme>`.
    pub fn set_theme(&mut self, theme: impl Into<Arc<TableTheme>>) {
        self.config.theme = theme.into();
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

    pub fn set_width_priority_columns(&mut self, columns: &[usize]) {
        self.config.width_priority_columns.clear();

        for &column in columns {
            if column < self.count_cols && !self.config.width_priority_columns.contains(&column) {
                self.config.width_priority_columns.push(column);
            }
        }
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
        self.styles.cell_colors.clear();
    }

    /// Converts a table to a String.
    ///
    /// It returns None in case where table cannot be fit to a terminal width.
    pub fn draw(self, termwidth: usize) -> Option<String> {
        build_table(self, termwidth)
    }

    /// Return a total table width.
    pub fn total_width(&self) -> usize {
        total_width(&self.widths, &self.config.theme)
    }

    /// Plans the column widths `draw` gives this table at `termwidth`, without
    /// drawing it. `None` when not even one column fits.
    pub(crate) fn plan(&self, termwidth: usize) -> Option<WidthEstimation> {
        let header = self
            .data
            .first()
            .filter(|_| self.config.structure.with_header);
        plan_columns(
            header.map(Vec::as_slice),
            &self.widths,
            [&self.deep_min_widths, &self.min_widths],
            &self.config,
            termwidth,
        )
    }

    /// Re-plans the visible columns of `plan` for the cells as they are now,
    /// with each `(column, width)` in `pinned` kept at least that wide. An
    /// expanded table uses it once its nested tables are drawn: a nested table
    /// that came out narrower than its column, or fell back to text, gives the
    /// room back to the other columns, while a drawn one keeps its width. The
    /// table never gets wider.
    pub(crate) fn replan(
        &self,
        plan: &WidthEstimation,
        pinned: &[(usize, usize)],
    ) -> WidthEstimation {
        let visible = plan.visible();
        let header = self
            .data
            .first()
            .filter(|_| self.config.structure.with_header && plan.header_on_border);
        let widths = &self.widths[..visible];
        let mut floors = column_floors(header.map(Vec::as_slice), widths, &[], &self.config);
        for (col, floor) in floors.iter_mut().enumerate() {
            // A column the plan squeezed below its floor stays squeezed.
            *floor = min(*floor, plan.needed[col]);
        }
        for &(col, width) in pinned.iter().filter(|(col, _)| *col < visible) {
            floors[col] = max(floors[col], width);
        }

        let room = plan.needed[..visible].iter().sum();
        let mut needed = shrink_columns(widths, &floors, &self.config, room);
        needed.extend_from_slice(&plan.needed[visible..]);

        WidthEstimation { needed, ..*plan }
    }

    /// Draws the table with the column widths of an earlier [`NuTable::plan`].
    ///
    /// Cells changed since then are wrapped or cut to the planned widths; the
    /// plan itself does not change.
    pub(crate) fn draw_planned(self, plan: WidthEstimation) -> Option<String> {
        build_planned_table(self, plan)
    }

    /// Widens column `col` to hold a nested table that is drawn later:
    /// `width` characters wide, drawn as a table down to `min_widths`
    /// characters (see [`NuTable::min_widths`]).
    pub(crate) fn reserve_width(&mut self, col: usize, width: usize, min_widths: (usize, usize)) {
        let pad = self.pad();
        self.widths[col] = max(self.widths[col], width + pad);
        self.min_widths[col] = max(self.min_widths[col], min_widths.0 + pad);
        self.deep_min_widths[col] = max(self.deep_min_widths[col], min_widths.1 + pad);
    }

    /// The padding of a cell: the width a column has beyond its content.
    pub(crate) fn pad(&self) -> usize {
        indent_sum(self.config.indent)
    }

    /// The widest content a cell of column `col` can get in a table at most
    /// `width` wide, with the columns before it (already filled in) at their
    /// floors and every column after it at least as narrow as `...`.
    pub(crate) fn cell_width_bound(&self, col: usize, width: usize) -> usize {
        let vertical = self.config.theme.as_base().borders_has_vertical() as usize;
        let pad = self.pad();
        let before = column_floors(None, &self.widths[..col], &[], &self.config);
        // The columns after it show, each at least its padding wide, or hide
        // behind the `...` column.
        let after = self.count_cols - col - 1;
        let after_width = min(
            after * (pad + vertical),
            trail_width(&self.config) + vertical,
        );
        let used = total_width(&before, &self.config.theme) + vertical * usize::from(col > 0);
        width.saturating_sub(used + pad + after_width)
    }

    /// The content width left for a cell of column `col` in a table drawn
    /// `width` wide with `plan`, once the columns before it take their width.
    pub(crate) fn cell_width_left(
        &self,
        plan: &WidthEstimation,
        col: usize,
        width: usize,
    ) -> usize {
        let vertical = self.config.theme.as_base().borders_has_vertical() as usize;
        let used = total_width(&plan.needed[..col], &self.config.theme);
        width.saturating_sub(used + vertical * usize::from(col > 0) + self.pad())
    }

    /// The narrowest widths this table draws at, with its first data column
    /// at its floor (its nested tables showing as text), and with the nested
    /// tables in its first data column drawn too. Neither is more than its
    /// natural width.
    pub(crate) fn min_widths(&self) -> (usize, usize) {
        let count = self.count_cols;
        let first_data = usize::from(self.config.structure.with_index && count > 1);
        let narrowest = |min_widths: &[usize]| {
            let floors = column_floors(None, &self.widths, min_widths, &self.config);
            let mut widths = floors[..=first_data].to_vec();
            if count > first_data + 1 {
                widths.push(trail_width(&self.config));
            }
            min(total_width(&widths, &self.config.theme), self.total_width())
        };

        (narrowest(&[]), narrowest(&self.deep_min_widths))
    }

    pub(crate) fn recalculate_dimensions(&mut self) {
        table_recalculate_widths(self);
        table_recalculate_heights(self);
    }

    /// Resets a cell to the default data alignment and color.
    pub(crate) fn reset_style(&mut self, pos: (usize, usize)) {
        let entity = Entity::Cell(pos.0, pos.1);
        let alignment = self.styles.alignments.data;
        self.styles.cell_alignments.push((entity, alignment));
        self.styles.cell_colors.push((entity, ANSIBuf::default()));
    }
}

// NOTE: Must never be called from nu-table - made only for tests
// FIXME: remove it?
// #[cfg(test)]
impl From<Vec<Vec<Text<String>>>> for NuTable {
    fn from(value: Vec<Vec<Text<String>>>) -> Self {
        let count_rows = value.len();
        let count_cols = if value.is_empty() { 0 } else { value[0].len() };

        let mut t = Self::new(count_rows, count_cols);
        for (i, row) in value.into_iter().enumerate() {
            t.set_row(i, row);
        }

        table_recalculate_widths(&mut t);

        t
    }
}

fn table_recalculate_widths(t: &mut NuTable) {
    let pad = indent_sum(t.config.indent);
    t.widths = build_width(&t.data, t.count_cols, t.count_rows, pad);
}

fn table_recalculate_heights(t: &mut NuTable) {
    t.heights = t
        .data
        .iter()
        .map(|row| row.iter().map(|cell| cell.count_lines()).max().unwrap_or(0))
        .collect();
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Hash)]
struct CellConfiguration<Value> {
    index: Value,
    header: Value,
    data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Styles {
    /// Colors and alignments set on cells (or on every cell), applied in this
    /// order when the table is drawn. A table keeps no grid configuration of
    /// its own until then, which keeps the many tables of `table --expand` small.
    cell_colors: Vec<(Entity, ANSIBuf)>,
    cell_alignments: Vec<(Entity, AlignmentHorizontal)>,
    colors: CellConfiguration<Color>,
    alignments: CellConfiguration<AlignmentHorizontal>,
}

impl Styles {
    /// The grid configuration holding every cell style.
    fn to_config(&self) -> ColoredConfig {
        let mut cfg = ColoredConfig::default();
        for (entity, color) in &self.cell_colors {
            cfg.set_color(*entity, color.clone());
        }
        for &(entity, alignment) in &self.cell_alignments {
            cfg.set_alignment_horizontal(entity, alignment);
        }
        cfg
    }

    /// Moves the cell styles one row up with their cells, dropping the first row's.
    fn remove_first_row(&mut self) {
        fn shift_up(entity: &mut Entity) -> bool {
            match entity {
                Entity::Cell(0, _) => false,
                Entity::Cell(row, _) => {
                    *row -= 1;
                    true
                }
                _ => true,
            }
        }
        self.cell_colors.retain_mut(|(entity, _)| shift_up(entity));
        self.cell_alignments
            .retain_mut(|(entity, _)| shift_up(entity));
    }
}

#[derive(Debug, Clone)]
pub struct TableConfig {
    theme: Arc<TableTheme>,
    trim: TrimStrategy,
    border_color: Option<Style>,
    expand: bool,
    structure: TableStructure,
    header_on_border: bool,
    indent: TableIndent,
    width_priority_columns: Vec<usize>,
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
    #[allow(dead_code)]
    align_index: AlignmentHorizontal,
    color: Option<Color>,
}

impl HeadInfo {
    fn new(
        values: Vec<String>,
        align: AlignmentHorizontal,
        align_index: AlignmentHorizontal,
        color: Option<Color>,
    ) -> Self {
        Self {
            values,
            align,
            align_index,
            color,
        }
    }
}

fn build_table(t: NuTable, termwidth: usize) -> Option<String> {
    if t.count_columns() == 0 || t.count_rows() == 0 {
        return Some(String::new());
    }

    let plan = t.plan(termwidth)?;
    build_planned_table(t, plan)
}

fn build_planned_table(mut t: NuTable, plan: WidthEstimation) -> Option<String> {
    if t.count_columns() == 0 || t.count_rows() == 0 {
        return Some(String::new());
    }

    // Cells may have changed since the plan was made: an expanded table draws
    // its nested tables into the planned widths. Wrap or cut against what the
    // cells hold now, the header row included.
    let current = t.widths.clone();
    table_truncate(&mut t, &plan);
    let head = remove_header_if(&mut t, plan.header_on_border);
    table_insert_footer_if(&mut t);

    draw_table(t, plan, current, head)
}

fn remove_header_if(t: &mut NuTable, on_border: bool) -> Option<HeadInfo> {
    if !on_border {
        return None;
    }

    let head = remove_header(t);
    t.config.structure.with_header = false;

    Some(head)
}

/// Whether the table is set up to draw its header on the top border (and the
/// footer on the bottom one), header text aside.
fn wants_header_on_border(cfg: &TableConfig) -> bool {
    let has_horizontal = cfg.theme.as_base().borders_has_top()
        || cfg.theme.as_base().get_horizontal_line(1).is_some();
    cfg.structure.with_header && cfg.header_on_border && has_horizontal
}

/// Whether a header cell can be drawn on a border.
///
/// The border holds one character per column of width, so a header with a
/// character that is not one column wide (CJK, emoji, a combining mark, a
/// newline), or whose characters join into fewer columns than they are (an
/// Arabic ligature), would shift the border: such a header keeps its own row.
fn fits_border(cell: &NuRecordsValue) -> bool {
    let text = cell.text();
    text.chars().all(|c| get_char_width(c) == 1) && string_width(text) == text.chars().count()
}

fn table_insert_footer_if(t: &mut NuTable) {
    let with_footer = t.config.structure.with_header && t.config.structure.with_footer;
    if !with_footer {
        return;
    }

    duplicate_row(&mut t.data, 0);

    if !t.heights.is_empty() {
        t.heights.push(t.heights[0]);
    }
}

/// Hides the columns `plan` left out behind the trailing `...` column.
fn table_truncate(t: &mut NuTable, plan: &WidthEstimation) {
    if !plan.trail {
        return;
    }

    let trail = plan.visible();
    truncate_rows(&mut t.data, trail);
    push_empty_column(&mut t.data);

    for row in 0..t.count_rows {
        t.reset_style((row, trail));
    }

    // Columns hidden behind `...` no longer set the row heights.
    table_recalculate_heights(t);
}

fn remove_header(t: &mut NuTable) -> HeadInfo {
    t.styles.remove_first_row();

    let head = t
        .data
        .remove(0)
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // drop height row
    t.heights.remove(0);

    let color = get_color_if_exists(&t.styles.colors.header);
    let alignment = t.styles.alignments.header;
    let alignment_index = if t.config.structure.with_index {
        t.styles.alignments.index
    } else {
        t.styles.alignments.header
    };

    t.styles.alignments.header = AlignmentHorizontal::Center;
    t.styles.colors.header = Color::empty();

    HeadInfo::new(head, alignment, alignment_index, color)
}

fn draw_table(
    t: NuTable,
    width: WidthEstimation,
    current: Vec<usize>,
    head: Option<HeadInfo>,
) -> Option<String> {
    let mut structure = t.config.structure;
    structure.with_footer = structure.with_footer && head.is_none();
    let sep_color = t.config.border_color;

    let data = t.data;
    let mut table = Builder::from_vec(data).build();

    set_styles(&mut table, t.styles, &structure);
    set_indent(&mut table, t.config.indent);
    load_theme(&mut table, &t.config.theme, &structure);
    let ctrl = DimensionCtrl::new(width, current, t.config.trim.clone(), &t.config, t.heights);
    table.with(ctrl);
    table_set_border_header(&mut table, head, &t.config);
    if let Some(style) = sep_color {
        color_borders(&mut table, style);
    }

    let string = table.to_string();
    Some(string)
}

fn set_styles(table: &mut Table, styles: Styles, structure: &TableStructure) {
    table.with(styles.to_config());
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

    // todo: Move logic to SetLineHeaders - so it be faster - cleaner
    if with_footer {
        let last_row = table.count_rows();
        table.with(SetLineHeaders::new(head.clone(), last_row, cfg.indent));
    }

    table.with(SetLineHeaders::new(head, 0, cfg.indent));
}

fn indent_sum(indent: TableIndent) -> usize {
    indent.left + indent.right
}

/// Width of the trailing `...` column, padding included.
fn trail_width(cfg: &TableConfig) -> usize {
    EMPTY_COLUMN_TEXT_WIDTH + indent_sum(cfg.indent)
}

fn set_indent(table: &mut Table, indent: TableIndent) {
    table.with(Padding::new(indent.left, indent.right, 0, 0));
}

/// Applies a plan: wraps or cuts the cells of the columns it made narrower
/// than they are, then fixes the widths and heights for drawing.
struct DimensionCtrl {
    width: WidthEstimation,
    /// Widths of the cells, one per column, before the plan is applied.
    current: Vec<usize>,
    /// Whether the plan made any visible column narrower than its cells.
    shrinks: bool,
    trim_strategy: TrimStrategy,
    pad: usize,
    heights: Vec<usize>,
}

impl DimensionCtrl {
    fn new(
        width: WidthEstimation,
        current: Vec<usize>,
        trim_strategy: TrimStrategy,
        cfg: &TableConfig,
        heights: Vec<usize>,
    ) -> Self {
        let shrinks = width.needed[..width.visible()]
            .iter()
            .zip(&current)
            .any(|(needed, current)| needed < current);
        Self {
            width,
            current,
            shrinks,
            trim_strategy,
            pad: indent_sum(cfg.indent),
            heights,
        }
    }
}

/// Column widths planned for a table, padding included.
#[derive(Debug, Clone)]
pub(crate) struct WidthEstimation {
    /// Widths to draw the visible columns at, then the trailing `...` column if `trail`.
    needed: Vec<usize>,
    trail: bool,
    /// Whether the header row is drawn on the top border.
    header_on_border: bool,
}

impl WidthEstimation {
    /// Number of visible columns, not counting the trailing `...` column.
    fn visible(&self) -> usize {
        self.needed.len() - self.trail as usize
    }

    /// Planned width of column `col`, padding included, or `None` when it is hidden.
    pub(crate) fn column_width(&self, col: usize) -> Option<usize> {
        (col < self.visible()).then(|| self.needed[col])
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimension> for DimensionCtrl {
    fn change(self, recs: &mut NuRecords, cfg: &mut ColoredConfig, dims: &mut CompleteDimension) {
        if self.shrinks {
            width_ctrl_truncate(self, recs, cfg, dims);
            return;
        }

        // NOTE: just an optimization; to not recalculate it internally
        dims.set_heights(self.heights);
        dims.set_widths(self.width.needed);
    }

    fn hint_change(&self) -> Option<Entity> {
        // NOTE:
        // Because we are assuming that:
        // len(lines(wrapped(string))) >= len(lines(string))
        //
        // Only truncation case must be relaclucated in term of height.
        if self.shrinks && matches!(self.trim_strategy, TrimStrategy::Truncate { .. }) {
            Some(Entity::Row(0))
        } else {
            None
        }
    }
}

fn width_ctrl_truncate(
    ctrl: DimensionCtrl,
    recs: &mut NuRecords,
    cfg: &mut ColoredConfig,
    dims: &mut CompleteDimension,
) {
    let mut heights = ctrl.heights;
    let visible = ctrl.width.visible();

    for (col, (&width, &current)) in ctrl.width.needed[..visible]
        .iter()
        .zip(&ctrl.current)
        .enumerate()
    {
        if width >= current {
            continue;
        }

        // Wrap and cut leave the cells that already fit alone, which keeps the
        // nested tables an expanded table drew into this width intact.
        let width = width.saturating_sub(ctrl.pad);
        let column = Entity::Column(col);
        match &ctrl.trim_strategy {
            TrimStrategy::Wrap { try_to_keep_words } => {
                if width < MIN_COLUMN_CONTENT_WIDTH {
                    // Wrapping at 1–3 columns turns "128 B" / "size" into a
                    // vertical ladder. Cut to one line instead.
                    let mut truncate = Width::truncate(width);
                    if width >= EMPTY_COLUMN_TEXT_WIDTH {
                        truncate = truncate.suffix(EMPTY_COLUMN_TEXT).suffix_try_color(true);
                    }
                    CellOption::<NuRecords, _>::change(truncate, recs, cfg, column);
                } else {
                    let wrap = Width::wrap(width).keep_words(*try_to_keep_words);
                    CellOption::<NuRecords, _>::change(wrap, recs, cfg, column);

                    // NOTE: An optimization to have proper heights without going over all the data again.
                    // We are going only for all rows in changed columns
                    for (row, row_height) in heights.iter_mut().enumerate() {
                        let height = recs.count_lines(Position::new(row, col));
                        *row_height = max(*row_height, height);
                    }
                }
            }
            TrimStrategy::Truncate { suffix } => {
                let mut truncate = Width::truncate(width);
                if let Some(suffix) = suffix {
                    truncate = truncate.suffix(suffix).suffix_try_color(true);
                }

                CellOption::<NuRecords, _>::change(truncate, recs, cfg, column);
            }
        }
    }

    if matches!(ctrl.trim_strategy, TrimStrategy::Truncate { .. }) {
        for (row, row_height) in heights.iter_mut().enumerate() {
            let mut height = 1;
            for col in 0..ctrl.width.needed.len() {
                height = max(height, recs.count_lines(Position::new(row, col)));
            }
            *row_height = height;
        }
    }

    dims.set_heights(heights);
    dims.set_widths(ctrl.width.needed);
}

fn align_table(
    table: &mut Table,
    alignments: CellConfiguration<AlignmentHorizontal>,
    structure: &TableStructure,
) {
    table.with(AlignmentStrategy::PerLine);

    if structure.with_header {
        table.modify(Rows::first(), AlignmentStrategy::PerCell);
        table.modify(Rows::first(), Alignment::from(alignments.header));

        if structure.with_footer {
            table.modify(Rows::last(), AlignmentStrategy::PerCell);
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

fn load_theme(table: &mut Table, theme: &TableTheme, structure: &TableStructure) {
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
}

/// Colors the borders with the separator style, once the table's text is final.
///
/// A colored border ends with a reset, which leaves the terminal at its default colors. With
/// the default separator style (the default foreground), only the borders that start a line
/// keep their color: every horizontal line, and the first vertical border of each cell line,
/// which is the left border or, in themes without one, the first inner border. Each line then
/// starts as it would with every border colored, whatever was printed before it, even when it
/// is printed on its own. From that border on the colors are the defaults, and a border or a
/// cell colored with the default foreground looks the same uncolored, as long as every cell
/// line ends with the defaults (see [`cells_to_uncolor`]). The table looks the same, line by
/// line, with fewer escape sequences.
fn color_borders(table: &mut Table, style: Style) {
    let color = ANSIBuf::from(convert_style(style));
    let count_rows = table.count_rows();
    let count_columns = table.count_columns();
    let first_vertical =
        (0..=count_columns).find(|&col| table.get_config().has_vertical(col, count_columns));
    let uncolored = first_vertical.and_then(|col| cells_to_uncolor(table, col));
    let cfg = table.get_config_mut();
    let (Some(first_vertical), Some(uncolored)) = (first_vertical, uncolored) else {
        cfg.set_border_color_default(color);
        return;
    };

    for pos in uncolored {
        cfg.set_color(pos.into(), ANSIBuf::default());
    }

    if style != nu_ansi_term::Color::Default.normal() {
        cfg.set_border_color_default(color);
        return;
    }

    let line = || Some(color.clone());
    cfg.set_borders_color(Borders {
        top: line(),
        top_left: line(),
        top_right: line(),
        top_intersection: line(),
        horizontal: line(),
        left_intersection: line(),
        right_intersection: line(),
        intersection: line(),
        bottom: line(),
        bottom_left: line(),
        bottom_right: line(),
        bottom_intersection: line(),
        // The left border is colored once for every row, an inner one row by row.
        left: (first_vertical == 0).then(|| color.clone()),
        ..Borders::default()
    });
    if first_vertical > 0 {
        for row in 0..count_rows {
            let border = Border {
                left: line(),
                ..Border::default()
            };
            cfg.set_border_color(Position::new(row, first_vertical), border);
        }
    }
}

/// The cells right of the first vertical border that are colored only with the default
/// foreground, if the terminal's colors are its defaults after every cell line as printed;
/// `None` if a cell line can end with other colors.
///
/// Cells left of the first vertical border keep their color, since nothing before them on
/// their line resets colors left on by whatever was printed before it. A colored cell line is
/// printed between the color's prefix and suffix, so it ends with the defaults when the suffix
/// is a reset. An uncolored one has to end with them by itself (see
/// [`ends_with_default_colors`]).
fn cells_to_uncolor(table: &Table, first_vertical: usize) -> Option<Vec<Position>> {
    let default_fg = ANSIBuf::from(convert_style(nu_ansi_term::Color::Default.normal()));
    let colors = table.get_config().get_colors();
    let records = table.get_records();
    let mut uncolored = Vec::new();
    for row in 0..table.count_rows() {
        for col in 0..table.count_columns() {
            let pos = Position::new(row, col);
            let ends_clean = || {
                records
                    .get_text(pos)
                    .split('\n')
                    .all(ends_with_default_colors)
            };
            match colors.get_color(pos).filter(|color| !color.is_empty()) {
                None if !ends_clean() => return None,
                Some(color) if color.get_suffix() != RESET => return None,
                Some(color) if col >= first_vertical && *color == default_fg && ends_clean() => {
                    uncolored.push(pos)
                }
                _ => {}
            }
        }
    }

    Some(uncolored)
}

const RESET: &str = "\x1b[0m";

/// Whether a line that starts with the terminal's default colors ends with them: it has no
/// escape sequence, or its last one is a reset. A C1 control sequence introducer (U+009B),
/// which some terminals read like `ESC [`, never counts as a reset.
fn ends_with_default_colors(line: &str) -> bool {
    line.rfind(['\x1b', '\u{9b}']).is_none_or(|i| {
        let last = &line[i..];
        last.starts_with(RESET) || last.starts_with("\x1b[m")
    })
}

/// Plans how the columns of a table fit `termwidth`.
///
/// `header` is the header row. It is drawn on the border when the table is
/// set up for that and every visible header fits there (see [`fits_border`]);
/// otherwise it keeps its own row. `None` when not even one column fits.
fn plan_columns(
    header: Option<&[NuRecordsValue]>,
    widths: &[usize],
    min_widths: [&[usize]; 2],
    cfg: &TableConfig,
    termwidth: usize,
) -> Option<WidthEstimation> {
    let header = header.filter(|_| wants_header_on_border(cfg));
    let plan = |header| plan_with_header(header, widths, min_widths, cfg, termwidth);
    let Some(header) = header else {
        return plan(None);
    };
    if header.iter().all(fits_border) {
        return plan(Some(header));
    }

    // A header that cannot sit on the border keeps the header row off it,
    // unless its column is hidden anyway.
    match plan(Some(header)) {
        Some(on_border)
            if on_border.header_on_border
                && header[..on_border.visible()].iter().all(fits_border) =>
        {
            Some(on_border)
        }
        _ => plan(None),
    }
}

/// Plans the columns with `header` on the border, or with no header there.
///
/// A table that fits keeps its natural `widths` (padding included); one set
/// to expand spreads the rest of `termwidth` evenly over its data columns.
///
/// Otherwise as many columns show as fit at their floors (see
/// [`column_floors`]), the rest hide from the right behind a trailing `...`
/// column, and the visible ones shrink as [`shrink_columns`] does. Columns
/// are taken from the left: a column holding nested tables takes the first of
/// `min_widths` (the width to draw them and the tables in their first column,
/// then the width to draw them) that fits there, which can hide the columns
/// to its right, and shows them as text otherwise. Marking a column
/// as a width priority never hides it: a visible priority column is meant to
/// keep a third of `termwidth` (or its natural width when that is less), so
/// columns to its right hide until it does or none is left.
///
/// When not even the first data column fits at its floor, it is squeezed
/// further, and the columns after it show as long as they fit at their floors,
/// up to as many as the narrowest table that fits at the floors shows; its
/// header leaves the border if it no longer fits there.
fn plan_with_header(
    header: Option<&[NuRecordsValue]>,
    widths: &[usize],
    min_widths: [&[usize]; 2],
    cfg: &TableConfig,
    termwidth: usize,
) -> Option<WidthEstimation> {
    let vertical = cfg.theme.as_base().borders_has_vertical() as usize;
    // The left and right borders, which is all a table without columns has.
    let outer = total_width(&[], &cfg.theme);
    let count = widths.len();
    // The index column alone tells nothing, so at least one data column shows.
    let first_data = usize::from(cfg.structure.with_index && count > 1);
    let mut header_on_border = header.is_some();

    let natural_total = total_width(widths, &cfg.theme);
    if count == 0 || natural_total <= termwidth {
        let mut needed = widths.to_vec();
        if cfg.expand {
            spread(
                &mut needed[first_data.min(count)..],
                termwidth - natural_total,
            );
        }
        return Some(WidthEstimation {
            needed,
            trail: false,
            header_on_border,
        });
    }

    let mut floors = column_floors(header, widths, &[], cfg);
    let trail_width = trail_width(cfg);
    // Width the borders of the first `k` columns take and, when columns are
    // hidden, the `...` column; then the width left for the columns themselves.
    let borders = |k: usize| {
        let trail = if k < count { trail_width + vertical } else { 0 };
        outer + vertical * (k - 1) + trail
    };
    let room = |k: usize| termwidth.checked_sub(borders(k));
    let fits = |k: usize, floors: &[usize]| {
        room(k).is_some_and(|room| floors[..k].iter().sum::<usize>() <= room)
    };

    // Columns are taken from the left, each at its floor. A column of nested
    // tables takes the widest width to draw them that fits, and shows them as
    // text otherwise; the first column that doesn't fit hides with the rest.
    let nested_floors = min_widths.map(|min_widths| column_floors(header, widths, min_widths, cfg));
    let mut taken = 0;
    let mut visible = 0;
    for col in 0..count {
        let rest: usize = floors[col + 1..].iter().sum();
        // With `...` after it, or with every column after it shown.
        let fits = |floor: usize| {
            taken + floor + borders(col + 1) <= termwidth
                || taken + floor + rest + borders(count) <= termwidth
        };
        let floor = nested_floors
            .iter()
            .map(|floors| floors[col])
            .find(|&floor| floor > floors[col] && fits(floor))
            .unwrap_or(floors[col]);
        if !fits(floor) {
            break;
        }
        floors[col] = floor;
        taken += floor;
        visible = col + 1;
    }

    let visible = match (visible > first_data).then_some(visible) {
        Some(visible) => make_room_for_priorities(visible, widths, &floors, cfg, termwidth, |k| {
            room(k).filter(|_| fits(k, &floors))
        }),
        None => {
            // No more columns than the narrowest table that fits at the
            // floors shows, so a wider terminal never shows fewer.
            let narrowest = (first_data + 1..=count)
                .map(|k| (borders(k) + floors[..k].iter().sum::<usize>(), Reverse(k)))
                .min()
                .map_or(first_data + 1, |(_, Reverse(k))| k);

            let pad = indent_sum(cfg.indent);
            let squeezed = min_column_width(MIN_COLUMN_CONTENT_WIDTH, pad, &cfg.trim);
            floors[first_data] = min(widths[first_data], squeezed);
            let header_width = header
                .and_then(|row| row.get(first_data))
                .map_or(0, |cell| cell.width() + pad);
            header_on_border &= header_width <= floors[first_data];

            (first_data + 1..=narrowest)
                .rev()
                .find(|&k| fits(k, &floors))?
        }
    };

    let room = room(visible)?;
    let mut needed = shrink_columns(&widths[..visible], &floors[..visible], cfg, room);
    let trail = visible < count;
    if trail {
        needed.push(trail_width);
    }

    Some(WidthEstimation {
        needed,
        trail,
        header_on_border,
    })
}

/// Hides columns right of the last of the first `visible` columns that is a
/// width priority, until every visible priority column can keep a third of
/// `termwidth` (or its natural width) with the other columns at their floors.
/// When no count achieves that, the count that leaves the priority columns the
/// most room wins. `room_if_fits(k)` is the room for `k` visible columns when
/// their floors fit in it.
///
/// Widths are compared in thirds, so rounding never makes a wider terminal
/// show fewer columns.
fn make_room_for_priorities(
    visible: usize,
    widths: &[usize],
    floors: &[usize],
    cfg: &TableConfig,
    termwidth: usize,
    room_if_fits: impl Fn(usize) -> Option<usize>,
) -> usize {
    let priorities = &cfg.width_priority_columns;
    let Some(last) = priorities
        .iter()
        .copied()
        .filter(|&col| col < visible)
        .max()
    else {
        return visible;
    };

    let others = |k: usize| -> usize {
        (0..k)
            .filter(|col| !priorities.contains(col))
            .map(|col| floors[col])
            .sum()
    };
    let wanted_thirds = |k: usize| -> usize {
        priorities
            .iter()
            .filter(|&&col| col < k)
            .map(|&col| min(3 * widths[col], max(3 * floors[col], termwidth)))
            .sum()
    };

    let counts: Vec<(usize, usize)> = (last + 1..=visible)
        .filter_map(|k| room_if_fits(k).map(|room| (k, room)))
        .collect();
    counts
        .iter()
        .rev()
        .find(|&&(k, room)| 3 * others(k) + wanted_thirds(k) <= 3 * room)
        .or_else(|| {
            counts
                .iter()
                .max_by_key(|&&(k, room)| (room - others(k), k))
        })
        .map_or(visible, |&(k, _)| k)
}

/// The narrowest width (padding included) each column may get before
/// columns are hidden: its natural width, at most [`MIN_READABLE_WIDTH`]
/// content characters, but at least the header when `header` sits on the
/// border (it cannot wrap there) and at least its `min_widths` entry.
fn column_floors(
    header: Option<&[NuRecordsValue]>,
    widths: &[usize],
    min_widths: &[usize],
    cfg: &TableConfig,
) -> Vec<usize> {
    let pad = indent_sum(cfg.indent);
    let base = min_column_width(MIN_READABLE_WIDTH, pad, &cfg.trim);

    widths
        .iter()
        .enumerate()
        .map(|(col, &natural)| {
            let header_width = header
                .and_then(|row| row.get(col))
                .map_or(0, |cell| cell.width() + pad);
            let min_width = min_widths.get(col).copied().unwrap_or(0);
            base.max(header_width).max(min_width).min(natural)
        })
        .collect()
}

fn min_column_width(base_min: usize, pad: usize, trim: &TrimStrategy) -> usize {
    match trim {
        TrimStrategy::Truncate { suffix } => {
            let suffix_width = suffix.as_deref().map(string_width).unwrap_or(0);
            max(base_min, suffix_width.saturating_add(1)) + pad
        }
        TrimStrategy::Wrap { .. } => base_min + pad,
    }
}

/// Shrinks `widths` to add up to `room`, never below `floors` (which must
/// fit): first the plain columns, then the width-priority columns, then the
/// index column (a record's keys). Within a tier the widest columns shrink
/// first, all to one shared width; a tier shrinks only once every tier before
/// it is at its floors.
fn shrink_columns(
    widths: &[usize],
    floors: &[usize],
    cfg: &TableConfig,
    room: usize,
) -> Vec<usize> {
    const LAST_TIER: usize = 2;
    let tier = |col: usize| {
        if col == 0 && cfg.structure.with_index {
            LAST_TIER
        } else {
            usize::from(cfg.width_priority_columns.contains(&col))
        }
    };

    let mut out = widths.to_vec();
    // The floors of the tiers already squeezed to them.
    let mut squeezed = 0;
    for t in 0..=LAST_TIER {
        let columns: Vec<usize> = (0..widths.len()).filter(|&col| tier(col) == t).collect();
        let later: usize = (0..widths.len())
            .filter(|&col| tier(col) > t)
            .map(|col| widths[col])
            .sum();
        let tier_floors: usize = columns.iter().map(|&col| floors[col]).sum();
        if t == LAST_TIER || squeezed + tier_floors + later <= room {
            level_columns(
                &mut out,
                &columns,
                floors,
                room.saturating_sub(squeezed + later),
            );
            break;
        }

        for &col in &columns {
            out[col] = floors[col];
        }
        squeezed += tier_floors;
    }

    out
}

/// Spreads `extra` evenly over `widths`; the leftmost get what does not divide.
fn spread(widths: &mut [usize], extra: usize) {
    let count = widths.len();
    if count == 0 {
        return;
    }
    for (i, width) in widths.iter_mut().enumerate() {
        *width += extra / count + usize::from(i < extra % count);
    }
}

/// Caps `columns` of `widths` at one shared width, the largest that keeps
/// their sum within `room`, never going below `floors`. The width left over
/// goes one character each to the leftmost capped columns.
fn level_columns(widths: &mut [usize], columns: &[usize], floors: &[usize], room: usize) {
    let natural: Vec<usize> = columns.iter().map(|&col| widths[col]).collect();
    let width_at = |i: usize, level: usize| max(floors[columns[i]], min(level, natural[i]));
    let sum_at = |level: usize| {
        (0..columns.len())
            .map(|i| width_at(i, level))
            .sum::<usize>()
    };

    let mut low = 0;
    let mut high = natural.iter().copied().max().unwrap_or(0);
    if sum_at(high) <= room {
        return;
    }

    // `sum_at` grows with the level: find the largest level that fits.
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if sum_at(mid) <= room {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    let level = low;
    let mut left = room.saturating_sub(sum_at(level));
    for (i, &col) in columns.iter().enumerate() {
        let mut width = width_at(i, level);
        if left > 0 && width == level && natural[i] > level {
            width += 1;
            left -= 1;
        }
        widths[col] = width;
    }
}

/// Width of a table with these column widths (padding included) and the
/// borders of `theme`.
pub(crate) fn total_width(widths: &[usize], theme: &TableTheme) -> usize {
    let theme = theme.as_base();
    let vertical = theme.borders_has_vertical() as usize;
    let outer = theme.borders_has_left() as usize + theme.borders_has_right() as usize;
    widths.iter().sum::<usize>() + vertical * widths.len().saturating_sub(1) + outer
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

fn build_width(
    records: &[Vec<NuRecordsValue>],
    count_cols: usize,
    count_rows: usize,
    pad: usize,
) -> Vec<usize> {
    // TODO: Expose not spaned version (could be optimized).
    let mut cfg = SpannedConfig::default();
    cfg.set_padding(
        Entity::Global,
        Sides::new(
            Indent::spaced(pad),
            Indent::zero(),
            Indent::zero(),
            Indent::zero(),
        ),
    );

    let records = IterRecords::new(records, count_cols, Some(count_rows));

    PeekableGridDimension::width(records, &cfg)
}

// It's laverages a use of guuaranted cached widths before hand
// to speed up things a bit.
struct SetLineHeaders {
    line: usize,
    pad: TableIndent,
    head: HeadInfo,
}

impl SetLineHeaders {
    fn new(head: HeadInfo, line: usize, pad: TableIndent) -> Self {
        Self { line, head, pad }
    }
}

impl TableOption<NuRecords, ColoredConfig, CompleteDimension> for SetLineHeaders {
    fn change(self, recs: &mut NuRecords, cfg: &mut ColoredConfig, dims: &mut CompleteDimension) {
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

        let pad = self.pad.left + self.pad.right;

        let columns = self
            .head
            .values
            .into_iter()
            .zip(widths.iter().cloned()) // it must be always safe to do
            .map(|(s, width)| {
                let content_width = width.saturating_sub(pad);
                if s == EMPTY_COLUMN_TEXT || string_width(&s) <= content_width {
                    s
                } else {
                    Truncate::truncate(&s, content_width).into_owned()
                }
            })
            .collect::<Vec<_>>();

        let mut names = ColumnNames::new(columns)
            .line(self.line)
            .alignment(Alignment::from(self.head.align));
        if let Some(color) = self.head.color {
            names = names.color(color);
        }

        //  FIXME: because of bug in tabled(latest) we got to modify columns
        //         because it fails to regognize right padding value
        //  UNCOMMENT when fixed

        // let alignment_head = Alignment::from(self.head.align);
        // let alignment_index = Alignment::from(self.head.align_index);
        // if self.head.align == self.head.align_index {
        //     names = names.alignment(alignment_head);
        // } else {
        //     let mut v = vec![alignment_head; widths.len()];
        //     v[0] = alignment_index;
        //     names = names.alignment(v);
        // }

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

pub fn get_color_if_exists(c: &Color) -> Option<Color> {
    if !is_color_empty(c) {
        Some(c.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table with the default separator style, the given theme and these rows of cell texts.
    fn colored_table(theme: TableTheme, rows: &[&[&str]]) -> NuTable {
        let mut table = NuTable::new(rows.len(), rows[0].len());
        table.set_theme(theme);
        table.set_structure(false, false, false);
        table.set_border_color(nu_ansi_term::Color::Default.normal());
        for (row, cells) in rows.iter().enumerate() {
            for (col, text) in cells.iter().enumerate() {
                table.insert((row, col), String::from(*text));
            }
        }
        table
    }

    /// Colors these cells with the default foreground.
    fn color_default_foreground(table: &mut NuTable, cells: &[(usize, usize)]) {
        let default_fg = TextStyle::new().style(nu_ansi_term::Color::Default.normal());
        for &pos in cells {
            table.insert_style(pos, default_fg);
        }
    }

    #[test]
    fn default_separator_colors_horizontal_lines_and_first_borders() {
        let table = colored_table(
            TableTheme::rounded(),
            &[&["a", "b"], &["\x1b[31mc\x1b[0m", "d\ne"]],
        );
        assert_eq!(
            table.draw(80).expect("fits"),
            "\x1b[39m╭───┬───╮\x1b[0m\n\
             \x1b[39m│\x1b[0m a │ b │\n\
             \x1b[39m│\x1b[0m \x1b[31mc\x1b[0m │ d │\n\
             \x1b[39m│\x1b[0m   │ e │\n\
             \x1b[39m╰───┴───╯\x1b[0m"
        );
    }

    #[test]
    fn cell_leaving_colors_on_colors_every_border() {
        let table = colored_table(TableTheme::rounded(), &[&["a", "b"], &["\x1b[41mc", "d"]]);
        assert_eq!(
            table.draw(80).expect("fits"),
            "\x1b[39m╭───┬───╮\x1b[0m\n\
             \x1b[39m│\x1b[0m a \x1b[39m│\x1b[0m b \x1b[39m│\x1b[0m\n\
             \x1b[39m│\x1b[0m \x1b[41mc \x1b[39m│\x1b[0m d \x1b[39m│\x1b[0m\n\
             \x1b[39m╰───┴───╯\x1b[0m"
        );
    }

    #[test]
    fn theme_without_left_border_colors_its_first_inner_border() {
        let mut table = colored_table(TableTheme::psql(), &[&["a", "b", "c"], &["d", "e", "f"]]);
        color_default_foreground(&mut table, &[(0, 0), (0, 1)]);
        assert_eq!(
            table.draw(80).expect("fits"),
            " \x1b[39ma\x1b[0m \x1b[39m|\x1b[0m b | c \n d \x1b[39m|\x1b[0m e | f "
        );
    }

    #[test]
    fn default_foreground_cells_are_uncolored_unless_they_leave_colors_on() {
        let mut table = colored_table(TableTheme::rounded(), &[&["a", "b"], &["\x1b[41mc", "d"]]);
        color_default_foreground(&mut table, &[(0, 0), (1, 0)]);
        assert_eq!(
            table.draw(80).expect("fits"),
            "\x1b[39m╭───┬───╮\x1b[0m\n\
             \x1b[39m│\x1b[0m a │ b │\n\
             \x1b[39m│\x1b[0m \x1b[39m\x1b[41mc\x1b[0m │ d │\n\
             \x1b[39m╰───┴───╯\x1b[0m"
        );
    }

    #[test]
    fn other_separator_colors_every_border_and_uncolors_default_foreground_cells() {
        let mut table = colored_table(TableTheme::rounded(), &[&["a", "b"], &["c", "d"]]);
        table.set_border_color(nu_ansi_term::Color::Red.normal());
        color_default_foreground(&mut table, &[(0, 0), (1, 0)]);
        assert_eq!(
            table.draw(80).expect("fits"),
            "\x1b[31m╭───┬───╮\x1b[0m\n\
             \x1b[31m│\x1b[0m a \x1b[31m│\x1b[0m b \x1b[31m│\x1b[0m\n\
             \x1b[31m│\x1b[0m c \x1b[31m│\x1b[0m d \x1b[31m│\x1b[0m\n\
             \x1b[31m╰───┴───╯\x1b[0m"
        );
    }

    #[test]
    fn lines_ending_with_default_colors() {
        let clean = ["", "plain", "\x1b[31mx\x1b[0m", "\x1b[31mx\x1b[m tail"];
        let dirty = [
            "\x1b[31mx",
            "\x1b[0m\x1b]8;;\x1b\\",
            "\x1b[1mx\x1b[22m",
            "x\x1b[0m\u{9b}41m",
            "\x1b[31mx\u{9b}0m",
        ];
        assert!(clean.into_iter().all(ends_with_default_colors));
        assert!(!dirty.into_iter().any(ends_with_default_colors));
    }
}
