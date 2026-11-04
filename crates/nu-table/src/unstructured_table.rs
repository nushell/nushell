use nu_color_config::StyleComputer;
use nu_protocol::{Config, Record, Span, TableIndent, Value};

use tabled::{
    grid::{
        ansi::ANSIStr,
        config::{Borders, CompactMultilineConfig},
        dimension::{DimensionPriority, PoolTableDimension},
        util::string::count_lines,
    },
    settings::{Alignment, Color, Padding, TableOption},
    tables::{PoolTable, TableValue},
};

use crate::{TableTheme, is_color_empty, string_width, string_wrap};

/// UnstructuredTable has a recursive table representation of nu_protocol::Value.
///
/// Text is always left-aligned. [`UnstructuredTable::truncate`] fits the table to a width by
/// wrapping text and dropping columns from the right, not by sizing columns.
pub struct UnstructuredTable {
    value: Node,
    indent: TableIndent,
}

/// The layout of an [`UnstructuredTable`]: tabled's [`TableValue`] plus [`Node::Table`].
///
/// A list of records has to line up in rows and columns, which a [`TableValue`] can't express:
/// a [`TableValue::Row`] of columns lets each column stack its own cell heights, and a
/// [`TableValue::Column`] of rows lets each row size its own cells. So a list of records stays a
/// [`Node::Table`] while it's truncated, which keeps or drops whole columns, and turns into rows
/// of equally wide cells only when it's drawn.
enum Node {
    Cell(String),
    Row(Vec<Node>),
    Column(Vec<Node>),
    /// A list of records: one [`Node::Column`] per field, holding the header and then the field's
    /// value in each record. Truncation may end it with a `...` cell instead of the columns it
    /// dropped.
    Table(Vec<Node>),
}

impl UnstructuredTable {
    pub fn new(value: Value, config: &Config) -> Self {
        let value = convert_nu_value_to_table_value(value, config);
        Self {
            value,
            indent: config.table.padding,
        }
    }

    /// Shrinks the table to fit in `width` columns. Returns `true` when not even a `...` fits.
    pub fn truncate(&mut self, theme: &TableTheme, width: usize) -> bool {
        let spacing = Spacing::new(theme, self.indent);
        let frame = spacing.borders.has_left() as usize + spacing.borders.has_right() as usize;
        let available = width.saturating_sub(frame);

        truncate_table_value(&mut self.value, &spacing, available).is_none()
    }

    pub fn draw(self, theme: &TableTheme, style: &StyleComputer) -> String {
        let spacing = Spacing::new(theme, self.indent);
        let mut value = spacing.table_value(self.value);

        if !spacing.has_vertical {
            // Without a left border, PoolTable (tabled 0.22) starts a blank line it adds under a
            // short cell with a stray space wherever the cell to its left has a row border, which
            // shifts the cell's column. Cells as tall as the space they get need no such lines.
            let height = spacing.height(&value);
            spacing.heighten(&mut value, height);
        }

        build_table(value, style, spacing.borders, self.indent)
    }
}

/// The padding and borders a [`PoolTable`] puts around its cells, needed to measure a
/// [`TableValue`] the way the [`PoolTable`] will lay it out.
struct Spacing {
    /// Left plus right padding of a cell.
    pad: usize,
    /// The borders the [`PoolTable`] draws.
    borders: Borders<char>,
    /// Whether a vertical border separates the cells of a row.
    has_vertical: bool,
    /// Whether a horizontal border separates the values of a column.
    has_horizontal: bool,
}

impl Spacing {
    fn new(theme: &TableTheme, indent: TableIndent) -> Self {
        let mut borders = *theme.as_full().get_borders();
        if !borders.has_left() {
            // PoolTable draws the line between two cells as the second cell's left border, so
            // without a left border there's no column for the intersections on that line either.
            // Drawing them anyway (`restructured` has some) would push those lines one column
            // right of the cell's text.
            borders.top_intersection = None;
            borders.bottom_intersection = None;
            borders.intersection = None;
        }

        Self {
            pad: indent.left + indent.right,
            borders,
            has_vertical: borders.has_left(),
            has_horizontal: borders.has_top(),
        }
    }

    /// The width a text takes in a cell: [`PoolTable`] draws nothing for a cell with no width, not
    /// even its borders, so every cell is at least one column wide.
    fn text_width(&self, text: &str) -> usize {
        (string_width(text) + self.pad).max(1)
    }

    /// Turns `node` into the [`TableValue`] the [`PoolTable`] draws.
    fn table_value(&self, node: Node) -> TableValue {
        match node {
            Node::Cell(text) => self.cell(text),
            // Drawn like an empty text, which is simpler to measure.
            Node::Row(list) | Node::Column(list) | Node::Table(list) if list.is_empty() => {
                self.cell(String::new())
            }
            Node::Row(list) => TableValue::Row(self.table_values(list)),
            Node::Column(list) => TableValue::Column(self.table_values(list)),
            Node::Table(columns) => self.table_rows(columns),
        }
    }

    /// A text cell, given a space when it would have no width (see [`Spacing::text_width`]).
    fn cell(&self, mut text: String) -> TableValue {
        if self.pad == 0 && string_width(&text) == 0 {
            text.push(' ');
        }

        TableValue::Cell(text)
    }

    /// Turns each node of `list` into a [`TableValue`] (see [`Spacing::table_value`]).
    fn table_values(&self, list: Vec<Node>) -> Vec<TableValue> {
        list.into_iter()
            .map(|node| self.table_value(node))
            .collect()
    }

    /// Lays a list of records out row by row, so each row is as tall as its tallest cell, and
    /// widens every cell to its column's width, so the columns line up.
    fn table_rows(&self, columns: Vec<Node>) -> TableValue {
        let mut rows: Vec<Vec<TableValue>> = Vec::new();
        let mut dropped = Vec::new();
        for column in columns {
            let cells = match column {
                Node::Column(cells) => self.table_values(cells),
                // The `...` truncation left in place of the columns it dropped.
                node => {
                    dropped.push(self.table_value(node));
                    continue;
                }
            };

            let widths: Vec<usize> = cells.iter().map(|cell| self.width(cell)).collect();
            let width = widths.iter().copied().max().unwrap_or_default();
            rows.resize_with(rows.len().max(cells.len()), Vec::new);
            for (row, (mut cell, current)) in rows.iter_mut().zip(cells.into_iter().zip(widths)) {
                self.widen(&mut cell, current, width);
                row.push(cell);
            }
        }

        let table = (!rows.is_empty())
            .then(|| TableValue::Column(rows.into_iter().map(TableValue::Row).collect()));
        match table {
            Some(table) if dropped.is_empty() => table,
            // The `...` stands to the right of the kept columns, as tall as the whole table.
            table => TableValue::Row(table.into_iter().chain(dropped).collect()),
        }
    }

    /// The width [`PoolTable`] gives `value` before stretching it to fit its parent.
    fn width(&self, value: &TableValue) -> usize {
        match value {
            TableValue::Cell(text) => self.text_width(text),
            TableValue::Row(list) => {
                let borders = self.has_vertical as usize * list.len().saturating_sub(1);
                list.iter().map(|value| self.width(value)).sum::<usize>() + borders
            }
            TableValue::Column(list) => list
                .iter()
                .map(|value| self.width(value))
                .max()
                .unwrap_or_default(),
        }
    }

    /// Makes `value`, which [`Spacing::width`] measured as `current` columns, at least `width`
    /// wide, giving the extra width to the same cells [`PoolTable`] would
    /// (`DimensionPriority::Last`): a row's last cell, and a column's first value, as the
    /// [`PoolTable`] stretches the rest of the column to match.
    ///
    /// Measuring colored text is the slow part of drawing, so the caller passes in the width it
    /// already has, and each level measures only the one value it widens.
    fn widen(&self, value: &mut TableValue, current: usize, width: usize) {
        let extra = width.saturating_sub(current);
        if extra == 0 {
            return;
        }

        match value {
            // A one-line text is `current` wide, so `extra` more spaces make it `width` wide.
            TableValue::Cell(text) if !text.contains('\n') => {
                text.extend(std::iter::repeat_n(' ', extra));
            }
            TableValue::Cell(text) => string_append_to_width(text, width - self.pad),
            TableValue::Row(list) => {
                if let Some(last) = list.last_mut() {
                    let last_width = self.width(last);
                    self.widen(last, last_width, last_width + extra);
                }
            }
            TableValue::Column(list) => {
                if let Some(first) = list.first_mut() {
                    let first_width = self.width(first);
                    self.widen(first, first_width, width);
                }
            }
        }
    }

    /// The height [`PoolTable`] gives `value` before stretching it to fit its parent.
    fn height(&self, value: &TableValue) -> usize {
        match value {
            TableValue::Cell(text) => count_lines(text),
            TableValue::Row(list) => list
                .iter()
                .map(|value| self.height(value))
                .max()
                .unwrap_or_default(),
            TableValue::Column(list) => {
                let borders = self.has_horizontal as usize * list.len().saturating_sub(1);
                list.iter().map(|value| self.height(value)).sum::<usize>() + borders
            }
        }
    }

    /// Appends empty lines to the cells of `value` until each fills the lines [`PoolTable`] gives
    /// it once `value` is `height` lines tall: a row gives each of its values its whole height,
    /// and a column gives its extra lines to its last value (`DimensionPriority::Last`).
    fn heighten(&self, value: &mut TableValue, height: usize) {
        let extra = height.saturating_sub(self.height(value));
        match value {
            TableValue::Cell(text) => text.extend(std::iter::repeat_n('\n', extra)),
            TableValue::Row(list) => {
                for value in list {
                    self.heighten(value, height);
                }
            }
            TableValue::Column(list) => {
                if let Some((last, rest)) = list.split_last_mut() {
                    for value in rest {
                        let height = self.height(value);
                        self.heighten(value, height);
                    }

                    let height = self.height(last) + extra;
                    self.heighten(last, height);
                }
            }
        }
    }
}

fn build_table(
    val: TableValue,
    style: &StyleComputer,
    borders: Borders<char>,
    indent: TableIndent,
) -> String {
    let mut table = PoolTable::from(val);

    table.with(Padding::new(indent.left, indent.right, 0, 0));
    table.with(borders);
    table.with(Alignment::left());
    table.with(PoolTableDimension::new(
        DimensionPriority::Last,
        DimensionPriority::Last,
    ));

    if let Some(color) = get_border_color(style)
        && !is_color_empty(&color)
    {
        return build_table_with_border_color(table, color);
    }

    table.to_string()
}

fn convert_nu_value_to_table_value(value: Value, config: &Config) -> Node {
    match value {
        Value::Record { val, .. } => build_vertical_map(val.into_owned(), config),
        Value::List { vals, .. } => {
            let rebuild_array_as_map = is_valid_record(&vals) && count_columns_in_record(&vals) > 0;
            if rebuild_array_as_map {
                build_map_from_record(vals.into_owned(), config)
            } else {
                build_vertical_array(vals.into_owned(), config)
            }
        }
        value => build_string_value(value, config),
    }
}

fn build_string_value(value: Value, config: &Config) -> Node {
    const MAX_STRING_WIDTH: usize = 50;
    const WRAP_STRING_WIDTH: usize = 30;

    let mut text = value.to_abbreviated_string(config);
    if string_width(&text) > MAX_STRING_WIDTH {
        text = string_wrap(&text, WRAP_STRING_WIDTH, false);
    }

    Node::Cell(text)
}

fn build_vertical_map(record: Record, config: &Config) -> Node {
    let max_key_width = record
        .iter()
        .map(|(k, _)| string_width(k))
        .max()
        .unwrap_or(0);

    let mut rows = Vec::with_capacity(record.len());
    for (mut key, value) in record {
        string_append_to_width(&mut key, max_key_width);

        let value = convert_nu_value_to_table_value(value, config);

        let row = Node::Row(vec![Node::Cell(key), value]);
        rows.push(row);
    }

    Node::Column(rows)
}

/// Pads `text` with spaces until it's `width` wide.
///
/// The spaces go at the end of the last line, after any color codes, so that a multiline text
/// gets wide enough even when its last line isn't its widest.
fn string_append_to_width(text: &mut String, width: usize) {
    let last_line = text.rsplit('\n').next().unwrap_or_default();
    let rest = width.saturating_sub(string_width(last_line));
    text.extend(std::iter::repeat_n(' ', rest));
}

fn build_vertical_array(vals: Vec<Value>, config: &Config) -> Node {
    let map = vals
        .into_iter()
        .map(|val| convert_nu_value_to_table_value(val, config))
        .collect();

    Node::Column(map)
}

fn is_valid_record(vals: &[Value]) -> bool {
    if vals.is_empty() {
        return true;
    }

    let first_value = match &vals[0] {
        Value::Record { val, .. } => val,
        _ => return false,
    };

    for val in &vals[1..] {
        match val {
            Value::Record { val, .. } => {
                let equal = val.columns().eq(first_value.columns());
                if !equal {
                    return false;
                }
            }
            _ => return false,
        }
    }

    true
}

fn count_columns_in_record(vals: &[Value]) -> usize {
    match vals.iter().next() {
        Some(Value::Record { val, .. }) => val.len(),
        _ => 0,
    }
}

fn build_map_from_record(vals: Vec<Value>, config: &Config) -> Node {
    // assumes that we have a valid record structure (checked by is_valid_record)

    let mut columns: Vec<Vec<Node>> = get_columns_in_record(&vals)
        .into_iter()
        .map(|col| vec![Node::Cell(col)])
        .collect();

    for val in vals {
        let val = get_as_record(val);
        for (column, (_, val)) in columns.iter_mut().zip(val.into_owned()) {
            column.push(convert_nu_value_to_table_value(val, config));
        }
    }

    Node::Table(columns.into_iter().map(Node::Column).collect())
}

fn get_as_record(val: Value) -> nu_utils::SharedCow<Record> {
    match val {
        Value::Record { val, .. } => val,
        _ => unreachable!(),
    }
}

fn get_columns_in_record(vals: &[Value]) -> Vec<String> {
    match vals.iter().next() {
        Some(Value::Record { val, .. }) => val.columns().cloned().collect(),
        _ => vec![],
    }
}

/// Shrinks `value` to fit in `available` columns and returns the width it takes then, or `None`
/// when not even a `...` fits.
///
/// Text wraps, or turns into `...` when it's too narrow to wrap. A row keeps the cells that fit,
/// then a `...` cell in place of the rest, dropping more cells from its end if that's what it
/// takes to make room for the `...`.
fn truncate_table_value(value: &mut Node, spacing: &Spacing, available: usize) -> Option<usize> {
    const MIN_CONTENT_WIDTH: usize = 10;
    const TRUNCATE_CELL_WIDTH: usize = 3;

    let truncate_cell_width = TRUNCATE_CELL_WIDTH + spacing.pad;

    match value {
        Node::Row(list) | Node::Column(list) | Node::Table(list) if list.is_empty() => {
            let width = spacing.text_width("");
            (width <= available).then_some(width)
        }
        Node::Row(row) | Node::Table(row) => {
            if row.len() == 1 {
                return truncate_table_value(&mut row[0], spacing, available);
            }

            // The width each kept cell takes, with the border after it.
            let mut widths = Vec::with_capacity(row.len());
            let mut row_width = 0;
            let count_cells = row.len();
            for (i, cell) in row.iter_mut().enumerate() {
                let border = (spacing.has_vertical && i + 1 != count_cells) as usize;
                let Some(rest) = available.checked_sub(row_width + border) else {
                    break;
                };
                let Some(width) = truncate_table_value(cell, spacing, rest) else {
                    break;
                };

                row_width += width + border;
                widths.push(width + border);
            }

            if widths.len() == count_cells {
                return Some(row_width);
            }

            while row_width + truncate_cell_width > available {
                row_width -= widths.pop()?;
            }

            if widths.is_empty() {
                *value = Node::Cell(String::from("..."));
            } else {
                row.truncate(widths.len());
                row.push(Node::Cell(String::from("...")));
            }

            Some(row_width + truncate_cell_width)
        }
        Node::Column(column) => {
            let mut max_width = 0;
            for cell in column.iter_mut() {
                let width = truncate_table_value(cell, spacing, available)?;
                max_width = std::cmp::max(max_width, width);
            }

            Some(max_width)
        }
        Node::Cell(text) => {
            // A cell keeps at least one column for its text.
            let room = available
                .checked_sub(spacing.pad)
                .filter(|&room| room > 0)?;
            let width = string_width(text);

            if width <= room {
                // The same as `spacing.text_width(text)`, without measuring the text again.
                Some((width + spacing.pad).max(1))
            } else if room > MIN_CONTENT_WIDTH {
                *text = string_wrap(text, room, false);
                Some(room + spacing.pad)
            } else if room >= TRUNCATE_CELL_WIDTH {
                *text = String::from("...");
                Some(truncate_cell_width)
            } else {
                // situation where we have too little space
                None
            }
        }
    }
}

fn build_table_with_border_color(mut table: PoolTable, color: Color) -> String {
    // NOTE: We have this function presizely because of color_into_ansistr internals
    // color must be alive  why we build table

    let color = color_into_ansistr(&color);
    table.with(SetBorderColor(color));
    table.to_string()
}

fn color_into_ansistr(color: &Color) -> ANSIStr<'static> {
    // # SAFETY
    //
    // It's perfectly save to do cause table does not store the reference internally.
    // We just need this unsafe section to cope with some limitations of [`PoolTable`].
    // Mitigation of this is definitely on a todo list.

    let prefix = color.get_prefix();
    let suffix = color.get_suffix();
    let prefix: &'static str = unsafe { std::mem::transmute(prefix) };
    let suffix: &'static str = unsafe { std::mem::transmute(suffix) };

    ANSIStr::new(prefix, suffix)
}

struct SetBorderColor(ANSIStr<'static>);

impl<R, D> TableOption<R, CompactMultilineConfig, D> for SetBorderColor {
    fn change(self, _: &mut R, cfg: &mut CompactMultilineConfig, _: &mut D) {
        let borders = Borders::filled(self.0);
        cfg.set_borders_color(borders);
    }
}

fn get_border_color(style: &StyleComputer<'_>) -> Option<Color> {
    // color_config closures for "separator" are just given a null.
    let color = style.compute("separator", &Value::nothing(Span::unknown()));
    let color = color.paint(" ").to_string();
    let color = Color::try_from(color);
    color.ok()
}
