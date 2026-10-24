use std::{cmp::max, sync::Arc};

use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_engine::column::get_columns;
use nu_protocol::{Config, Record, ShellError, Span, Value};

use crate::{
    NuTable, StringResult, TableOpts, TableOutput, TableTheme,
    common::{
        INDEX_COLUMN_NAME, NuText, check_value, configure_table_with_theme, error_sign,
        get_header_style, get_index_style, is_footer_needed, load_theme, nu_value_to_string,
        nu_value_to_string_clean, nu_value_to_string_colored, wrap_text,
    },
    string_width,
    table::{WidthEstimation, total_width},
    types::{get_table_row_index, has_index, resolve_priority_columns},
};

/// An expanded table first turns values into cells, recording the natural and
/// the minimum width of every column (nested tables included), then draws
/// top-down: a table plans its columns for the width it gets and draws each
/// nested table into the width of its column, with the same rules. Once its
/// nested tables are drawn, a table re-plans its visible columns once: a nested
/// table that came out narrower than its column, or fell back to text, gives
/// the room back to the other columns.
///
/// A nested table is only built when the width it could get at most leaves
/// room for one: the width each column could get follows from the columns
/// before it at their floors.
#[derive(Debug, Clone)]
pub struct ExpandedTable {
    expand_limit: Option<usize>,
    flatten: bool,
    flatten_sep: String,
}

impl ExpandedTable {
    pub fn new(expand_limit: Option<usize>, flatten: bool, flatten_sep: String) -> Self {
        Self {
            expand_limit,
            flatten,
            flatten_sep,
        }
    }

    pub fn build_value(self, item: &Value, opts: TableOpts<'_>) -> NuText {
        let width = opts.width;
        let cfg = Cfg::new(opts, self, false);

        match build_cell(item, &cfg, CellKind::ListItem, width) {
            Cell::Text((text, style)) => (wrap_text(&text, width, cfg.opts.config), style),
            Cell::Table {
                layout,
                value,
                kind,
            } => match layout.draw_stretched(width, width, &cfg) {
                Some(drawn) => (drawn.text, TextStyle::default()),
                None => {
                    let (text, style) = fallback_text(value, kind, &cfg);
                    (wrap_text(&text, width, cfg.opts.config), style)
                }
            },
        }
    }

    pub fn build_map(self, record: &Record, opts: TableOpts<'_>) -> StringResult {
        let width = opts.width;
        let cfg = Cfg::new(opts, self, false);

        let layout = record_layout(record, &cfg, width)?;
        let drawn = layout.draw_stretched(width, width, &cfg);
        // Drawing stops on an interrupt: report it instead of a partial table.
        cfg.opts.signals.check(&cfg.opts.span)?;
        Ok(drawn.map(|drawn| drawn.text))
    }

    pub fn build_list(self, vals: &[Value], opts: TableOpts<'_>) -> StringResult {
        let width = opts.width;
        let cfg = Cfg::new(opts, self, true);

        let Some(layout) = list_layout(vals, &cfg, width)? else {
            return Ok(None);
        };
        let drawn = layout.draw_stretched(width, width, &cfg);
        // Drawing stops on an interrupt: report it instead of a partial table.
        cfg.opts.signals.check(&cfg.opts.span)?;
        Ok(drawn.map(|drawn| drawn.text))
    }
}

#[derive(Debug, Clone)]
struct Cfg<'a> {
    opts: TableOpts<'a>,
    format: ExpandedTable,
    /// When true, a `Value::Error` row is re-raised (top-level `table --expand`).
    /// Nested expansion leaves errors as cells so wrapping records (e.g. `$ans`) stay printable.
    raise_row_errors: bool,
    /// The theme every table of the expansion shares.
    theme: Arc<TableTheme>,
}

impl<'a> Cfg<'a> {
    fn new(opts: TableOpts<'a>, format: ExpandedTable, raise_row_errors: bool) -> Self {
        let theme = Arc::new(load_theme(opts.mode));
        Self {
            opts,
            format,
            raise_row_errors,
            theme,
        }
    }
}

/// A table whose nested tables are drawn once the widths of their columns are known.
struct Layout<'v> {
    /// The table with every text cell filled in. A nested table's cell stays
    /// empty until it is drawn, but its column is already as wide as the
    /// nested table, and no narrower than the nested table can be drawn.
    out: TableOutput,
    nested: Vec<Nested<'v>>,
    /// A record table: keys in the first column, values in the second.
    record: bool,
}

struct Nested<'v> {
    pos: (usize, usize),
    layout: Box<Layout<'v>>,
    /// The value the table shows, and how, for the text shown instead when
    /// the table does not fit its column, e.g. `[table 3 rows]`.
    value: &'v Value,
    kind: CellKind,
    /// The key of a record value, moved down to the first row of the nested table.
    key: Option<String>,
}

/// A drawn table.
struct Drawn {
    text: String,
    /// Rows of the table and of the nested tables drawn in it, counted for
    /// `footer_inheritance`.
    rows: usize,
}

impl Layout<'_> {
    /// Draws the table at most `width` wide, or returns `None` when not even
    /// one column fits or drawing was interrupted.
    fn draw(self, width: usize, cfg: &Cfg<'_>) -> Option<Drawn> {
        let plan = self.out.table.plan(width)?;
        self.draw_planned(plan, width, cfg, true)
    }

    /// Draws the table stretched to `width` when it already fills most of
    /// `room` (`width` itself, or the room a nested table's cell has once the
    /// columns before it are drawn): the data columns (a record's values, a
    /// list's columns) share the rest.
    fn draw_stretched(mut self, width: usize, room: usize, cfg: &Cfg<'_>) -> Option<Drawn> {
        const EXPAND_THRESHOLD: f32 = 0.80;

        let total_width = self.out.table.total_width();
        let fills_most = total_width < width && total_width as f32 / room as f32 > EXPAND_THRESHOLD;
        if !fills_most {
            return self.draw(width, cfg);
        }

        self.out.table.set_strategy(true);
        let plan = self.out.table.plan(width)?;
        // The stretched width is the point: no re-plan narrows it again.
        self.draw_planned(plan, width, cfg, false)
    }

    /// Draws the table `width` wide at most with the column widths of `plan`,
    /// nested tables included, re-planning once they are drawn when `replan`
    /// is set.
    fn draw_planned(
        self,
        plan: WidthEstimation,
        width: usize,
        cfg: &Cfg<'_>,
        replan: bool,
    ) -> Option<Drawn> {
        let Layout {
            mut out, nested, ..
        } = self;
        let table = &mut out.table;
        let pad = table.pad();
        // Columns holding drawn nested tables, at the widest of them: they
        // cannot change width once drawn.
        let mut pinned: Vec<(usize, usize)> = vec![];
        // Rows the nested tables of each column add.
        let mut nested_rows = vec![0usize; table.count_columns()];

        for nested in nested {
            if cfg.opts.signals.interrupted() {
                return None;
            }

            let Nested {
                pos,
                layout,
                value,
                kind,
                key,
            } = nested;
            let Some(column_width) = plan.column_width(pos.1) else {
                continue;
            };

            let cell_width = column_width.saturating_sub(pad);
            // A nested record that fills most of the width left for it, once
            // the columns before it are drawn, fills its cell like the outer
            // table; a nested list does not.
            let drawn = if layout.record {
                let room = table.cell_width_left(&plan, pos.1, width);
                layout.draw_stretched(cell_width, room, cfg)
            } else {
                layout.draw(cell_width, cfg)
            };
            // A nested table is drawn into its cell, so it never comes out
            // wider; if one did, its text keeps the table around it whole.
            let drawn = drawn.filter(|drawn| {
                let fits = string_width(&drawn.text) <= cell_width;
                debug_assert!(fits, "a nested table came out wider than its cell");
                fits
            });

            match drawn {
                Some(Drawn { text, rows }) => {
                    if let Some(key) = key {
                        table.insert((pos.0, 0), format!("\n{key}"));
                    }

                    let width = string_width(&text) + pad;
                    match pinned.iter_mut().find(|(col, _)| *col == pos.1) {
                        Some((_, pinned)) => *pinned = max(*pinned, width),
                        None => pinned.push((pos.1, width)),
                    }
                    nested_rows[pos.1] += rows.saturating_sub(1);
                    table.insert(pos, text);
                }
                None => {
                    let (text, style) = fallback_text(value, kind, cfg);
                    table.insert(pos, text);
                    table.reset_style(pos);
                    table.insert_style(pos, style);
                }
            }
        }

        table.recalculate_dimensions();
        let plan = if replan {
            table.replan(&plan, &pinned)
        } else {
            plan
        };

        let data_rows = table.count_rows() - usize::from(out.with_header);
        out.count_rows = data_rows + nested_rows.into_iter().max().unwrap_or(0);
        let with_footer = is_footer_needed(cfg.opts.config, &out);
        out.table
            .set_structure(out.with_index, out.with_header, with_footer);

        let rows = out.count_rows;
        let text = out.table.draw_planned(plan)?;
        Some(Drawn { text, rows })
    }
}

/// A cell before drawing: text, or a nested table.
enum Cell<'v> {
    Text(NuText),
    Table {
        layout: Box<Layout<'v>>,
        value: &'v Value,
        kind: CellKind,
    },
}

/// List items and record values are shown differently: a record value is colored
/// in its text and left-aligned, and is never flattened.
#[derive(Clone, Copy, PartialEq)]
enum CellKind {
    ListItem,
    RecordValue,
}

/// Text for a list or record that is not drawn as a table.
fn fallback_text(value: &Value, kind: CellKind, cfg: &Cfg<'_>) -> NuText {
    let (text, style) = nu_value_to_string(value, cfg.opts.config, &cfg.opts.style_computer);
    match kind {
        CellKind::ListItem => (text, style),
        CellKind::RecordValue => (text, TextStyle::default()),
    }
}

/// Lays out a list as a table at most `width` wide.
fn list_layout<'v>(
    vals: &'v [Value],
    cfg: &Cfg<'_>,
    width: usize,
) -> Result<Option<Layout<'v>>, ShellError> {
    if vals.is_empty() {
        return Ok(None);
    }

    let headers = get_columns(vals);
    let with_index = has_index(&cfg.opts, &headers);
    // The INDEX header is dropped: its values fill the index column instead.
    let headers: Vec<_> = headers
        .into_iter()
        .filter(|header| !with_index || header != INDEX_COLUMN_NAME)
        .collect();
    let with_header = !headers.is_empty();
    let first_row = with_header as usize;
    let first_column = with_index as usize;
    let count_data_columns = max(headers.len(), 1);

    let mut table = NuTable::new(vals.len() + first_row, count_data_columns + first_column);
    table.set_index_style(get_index_style(&cfg.opts.style_computer));
    table.set_header_style(get_header_style(&cfg.opts.style_computer));
    table.set_indent(cfg.opts.config.table.padding);
    // Width priorities from pipeline metadata name columns of the outer table only.
    let priorities =
        resolve_priority_columns(&headers, &cfg.opts.width_priority_columns, with_index);
    table.set_width_priority_columns(&priorities);

    let mut out = TableOutput::new(table, with_header, with_index, vals.len());
    configure(&mut out, cfg);
    let table = &mut out.table;

    if with_index {
        if with_header {
            table.insert((0, 0), String::from("#"));
        }

        for (row, item) in vals.iter().enumerate() {
            cfg.opts.signals.check(&cfg.opts.span)?;
            cfg.check_row(item)?;

            let index = get_table_row_index(item, cfg.opts.config, row, cfg.opts.index_offset);
            table.insert((row + first_row, 0), index);
        }
    }

    let mut nested = vec![];
    for col in 0..count_data_columns {
        let column = col + first_column;
        let header = headers.get(col);
        if let Some(header) = header {
            table.insert((0, column), header.clone());
        }

        // The columns before this one are filled in: at their floors, they
        // leave this one at most this much.
        let cell_width = table.cell_width_bound(column, width);
        for (row, item) in vals.iter().enumerate() {
            cfg.opts.signals.check(&cfg.opts.span)?;
            cfg.check_row(item)?;

            let cell = match header {
                Some(header) => build_cell_with_header(item, header, cfg, cell_width),
                None => build_cell(item, cfg, CellKind::ListItem, cell_width),
            };
            insert_cell(table, &mut nested, (row + first_row, column), cell, None);
        }
    }

    Ok(Some(Layout {
        out,
        nested,
        record: false,
    }))
}

/// Lays out a record as a key-value table at most `width` wide.
fn record_layout<'v>(
    record: &'v Record,
    cfg: &Cfg<'_>,
    width: usize,
) -> Result<Layout<'v>, ShellError> {
    let mut table = NuTable::new(record.len(), 2);
    table.set_index_style(get_key_style(cfg));
    table.set_indent(cfg.opts.config.table.padding);

    let mut out = TableOutput::new(table, false, true, record.len());
    configure(&mut out, cfg);
    let table = &mut out.table;

    // The keys come first: they bound the width left for the values.
    for (row, key) in record.columns().enumerate() {
        table.insert((row, 0), key.clone());
    }
    let cell_width = table.cell_width_bound(1, width);

    // A key sits beside the first row of its nested table, below the top border.
    let key_beside_first_row = cfg.theme.as_base().borders_has_top();

    let mut nested = vec![];
    for (row, (key, value)) in record.iter().enumerate() {
        cfg.opts.signals.check(&cfg.opts.span)?;

        let cell = build_cell(value, cfg, CellKind::RecordValue, cell_width);
        let moved_key = (key_beside_first_row && !key.is_empty()).then(|| key.clone());
        insert_cell(table, &mut nested, (row, 1), cell, moved_key);
    }

    Ok(Layout {
        out,
        nested,
        record: true,
    })
}

/// Sets up a table of the expansion: theme, trim, header placement. Its footer
/// is decided when it is drawn, once the rows of its nested tables are known.
fn configure(out: &mut TableOutput, cfg: &Cfg<'_>) {
    configure_table_with_theme(
        out,
        cfg.opts.config,
        &cfg.opts.style_computer,
        cfg.opts.mode,
        Arc::clone(&cfg.theme),
    );
}

/// Puts `cell` at `pos`. A nested table only reserves the width of its column here.
fn insert_cell<'v>(
    table: &mut NuTable,
    nested: &mut Vec<Nested<'v>>,
    pos: (usize, usize),
    cell: Cell<'v>,
    key: Option<String>,
) {
    match cell {
        Cell::Text((text, style)) => {
            table.insert(pos, text);
            table.insert_style(pos, style);
        }
        Cell::Table {
            layout,
            value,
            kind,
        } => {
            let natural = layout.out.table.total_width();
            table.reserve_width(pos.1, natural, layout.out.table.min_widths());
            table.insert_style(pos, TextStyle::default());
            nested.push(Nested {
                pos,
                layout,
                value,
                kind,
                key,
            });
        }
    }
}

fn build_cell_with_header<'v>(
    item: &'v Value,
    header: &str,
    cfg: &Cfg<'_>,
    width: usize,
) -> Cell<'v> {
    match item {
        Value::Record { val, .. } => match val.get(header) {
            Some(val) => build_cell(val, cfg, CellKind::ListItem, width),
            None => Cell::Text(error_sign(
                cfg.opts.config.table.missing_value_symbol.clone(),
                &cfg.opts.style_computer,
            )),
        },
        _ => build_cell(item, cfg, CellKind::ListItem, width),
    }
}

/// Turns `value` into a cell at most `width` wide. A list or record becomes a
/// nested table when it could be drawn that wide, and text otherwise.
fn build_cell<'v>(value: &'v Value, cfg: &Cfg<'_>, kind: CellKind, width: usize) -> Cell<'v> {
    let config = cfg.opts.config;
    let style_computer = &cfg.opts.style_computer;

    if is_limit_reached(cfg) {
        let (text, style) = nu_value_to_string_clean(value, config, style_computer);
        return match kind {
            CellKind::ListItem => Cell::Text((text, style)),
            CellKind::RecordValue => Cell::Text((text, TextStyle::default())),
        };
    }

    // The narrowest table there is: its borders around one empty column.
    let padding = config.table.padding;
    let narrowest = total_width(&[padding.left + padding.right], &cfg.theme);
    let fits = width >= narrowest;
    let inner_cfg = || cfg_expand_next_level(cfg.clone(), value.span());
    let layout = match value {
        Value::Record { val, .. } if !val.is_empty() && fits => {
            record_layout(val, &inner_cfg(), width).ok()
        }
        Value::List { vals, .. } => {
            if kind == CellKind::ListItem && cfg.format.flatten && is_simple_list(vals) {
                let text = list_to_string(vals, config, style_computer, &cfg.format.flatten_sep);
                return Cell::Text((text, TextStyle::default()));
            }

            fits.then(|| list_layout(vals, &inner_cfg(), width).ok().flatten())
                .flatten()
        }
        Value::Record { .. } => None,
        _ => {
            return match kind {
                CellKind::ListItem => {
                    Cell::Text(nu_value_to_string_clean(value, config, style_computer))
                }
                CellKind::RecordValue => {
                    let text = nu_value_to_string_colored(value, config, style_computer);
                    Cell::Text((text, TextStyle::default()))
                }
            };
        }
    };

    match layout {
        Some(layout) => Cell::Table {
            layout: Box::new(layout),
            value,
            kind,
        },
        None => Cell::Text(fallback_text(value, kind, cfg)),
    }
}

fn get_key_style(cfg: &Cfg<'_>) -> TextStyle {
    get_header_style(&cfg.opts.style_computer).alignment(Alignment::Left)
}

fn is_limit_reached(cfg: &Cfg<'_>) -> bool {
    matches!(cfg.format.expand_limit, Some(0))
}

fn is_simple_list(vals: &[Value]) -> bool {
    vals.iter()
        .all(|v| !matches!(v, Value::Record { .. } | Value::List { .. }))
}

fn list_to_string(
    vals: &[Value],
    config: &Config,
    style_computer: &StyleComputer,
    sep: &str,
) -> String {
    let mut buf = String::new();
    for (i, value) in vals.iter().enumerate() {
        if i > 0 {
            buf.push_str(sep);
        }

        let (text, _) = nu_value_to_string_clean(value, config, style_computer);
        buf.push_str(&text);
    }

    buf
}

impl Cfg<'_> {
    fn check_row(&self, item: &Value) -> Result<(), ShellError> {
        if self.raise_row_errors {
            check_value(item)?;
        }
        Ok(())
    }
}

fn cfg_expand_next_level(mut cfg: Cfg<'_>, span: Span) -> Cfg<'_> {
    cfg.opts.span = span;
    cfg.opts.index_offset = 0;
    cfg.opts.width_priority_columns.clear();
    cfg.raise_row_errors = false;
    if let Some(deep) = cfg.format.expand_limit.as_mut() {
        *deep -= 1
    }

    cfg
}
