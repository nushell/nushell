use crate::data::value::{format_leaf, style_leaf};
use crate::format::RenderView;
use crate::prelude::*;
use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{UntaggedValue, Value};
use textwrap::fill;

use prettytable::format::{Alignment, FormatBuilder, LinePosition, LineSeparator};
use prettytable::{color, Attr, Cell, Row, Table};

type Entries = Vec<Vec<(String, &'static str)>>;

#[derive(Debug, new)]
pub struct TableView {
    // List of header cell values:
    headers: Vec<String>,

    // List of rows of cells, each containing value and prettytable style-string:
    entries: Entries,
}

enum TableMode {
    Light,
    Normal,
}

impl TableView {
    pub fn from_list(values: &[Value], starting_idx: usize) -> Option<TableView> {
        if values.is_empty() {
            return None;
        }

        // Different platforms want different amounts of buffer, not sure why
        let termwidth = std::cmp::max(textwrap::termwidth(), 20);

        let mut headers = nu_protocol::merge_descriptors(values);
        let mut entries = values_to_entries(values, &mut headers, starting_idx);
        let max_per_column = max_per_column(&headers, &entries, values.len());

        maybe_truncate_columns(&mut headers, &mut entries, termwidth);
        let headers_len = headers.len();

        // Measure how big our columns need to be (accounting for separators also)
        let max_naive_column_width = (termwidth - 3 * (headers_len - 1)) / headers_len;

        let column_space =
            ColumnSpace::measure(&max_per_column, max_naive_column_width, headers_len);

        // This gives us the max column width
        let max_column_width = column_space.max_width(termwidth);

        // This width isn't quite right, as we're rounding off some of our space
        let column_space = column_space.fix_almost_column_width(
            &max_per_column,
            max_naive_column_width,
            max_column_width,
            headers_len,
        );

        // This should give us the final max column width
        let max_column_width = column_space.max_width(termwidth);

        // Wrap cells as needed
        let table_view = wrap_cells(
            headers,
            entries,
            max_per_column,
            max_naive_column_width,
            max_column_width,
        );
        Some(table_view)
    }
}

fn are_table_indexes_disabled() -> bool {
    let config = crate::data::config::config(Tag::unknown());
    match config {
        Ok(config) => {
            let disable_indexes = config.get("disable_table_indexes");
            disable_indexes.map_or(false, |x| x.as_bool().unwrap_or(false))
        }
        _ => false,
    }
}

fn values_to_entries(values: &[Value], headers: &mut Vec<String>, starting_idx: usize) -> Entries {
    let disable_indexes = are_table_indexes_disabled();
    let mut entries = vec![];

    if headers.is_empty() {
        headers.push("".to_string());
    }

    for (idx, value) in values.iter().enumerate() {
        let mut row: Vec<(String, &'static str)> = headers
            .iter()
            .map(|d: &String| {
                if d == "" {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => (
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing()),
                        ),
                        _ => (format_leaf(value).plain_string(100_000), style_leaf(value)),
                    }
                } else {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => {
                            let data = value.get_data(d);
                            (
                                format_leaf(data.borrow()).plain_string(100_000),
                                style_leaf(data.borrow()),
                            )
                        }
                        _ => (
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing()),
                        ),
                    }
                }
            })
            .collect();

        // Indices are green, bold, right-aligned:
        if !disable_indexes {
            row.insert(0, ((starting_idx + idx).to_string(), "Fgbr"));
        }

        entries.push(row);
    }

    if !disable_indexes {
        headers.insert(0, "#".to_owned());
    }

    entries
}

#[allow(clippy::ptr_arg)]
fn max_per_column(headers: &[String], entries: &Entries, values_len: usize) -> Vec<usize> {
    let mut max_per_column = vec![];

    for i in 0..headers.len() {
        let mut current_col_max = 0;
        let iter = entries.iter().take(values_len);

        for entry in iter {
            let value_length = entry[i].0.chars().count();
            if value_length > current_col_max {
                current_col_max = value_length;
            }
        }

        max_per_column.push(std::cmp::max(current_col_max, headers[i].chars().count()));
    }

    max_per_column
}

fn maybe_truncate_columns(headers: &mut Vec<String>, entries: &mut Entries, termwidth: usize) {
    // Make sure we have enough space for the columns we have
    let max_num_of_columns = termwidth / 10;

    // If we have too many columns, truncate the table
    if max_num_of_columns < headers.len() {
        headers.truncate(max_num_of_columns);

        for entry in entries.iter_mut() {
            entry.truncate(max_num_of_columns);
        }

        headers.push("...".to_owned());

        for entry in entries.iter_mut() {
            entry.push(("...".to_owned(), "c")); // ellipsis is centred
        }
    }
}

struct ColumnSpace {
    num_overages: usize,
    underage_sum: usize,
    overage_separator_sum: usize,
}

impl ColumnSpace {
    /// Measure how much space we have once we subtract off the columns who are small enough
    fn measure(
        max_per_column: &[usize],
        max_naive_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut underage_sum = 0;
        let mut overage_separator_sum = 0;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                num_overages += 1;
                if i != (headers_len - 1) {
                    overage_separator_sum += 3;
                }
                if i == 0 {
                    overage_separator_sum += 1;
                }
            } else {
                underage_sum += column_max;
                // if column isn't last, add 3 for its separator
                if i != (headers_len - 1) {
                    underage_sum += 3;
                }
                if i == 0 {
                    underage_sum += 1;
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn fix_almost_column_width(
        self,
        max_per_column: &[usize],
        max_naive_column_width: usize,
        max_column_width: usize,
        headers_len: usize,
    ) -> ColumnSpace {
        let mut num_overages = 0;
        let mut overage_separator_sum = 0;
        let mut underage_sum = self.underage_sum;
        let iter = max_per_column.iter().enumerate().take(headers_len);

        for (i, &column_max) in iter {
            if column_max > max_naive_column_width {
                if column_max <= max_column_width {
                    underage_sum += column_max;
                    // if column isn't last, add 3 for its separator
                    if i != (headers_len - 1) {
                        underage_sum += 3;
                    }
                    if i == 0 {
                        underage_sum += 1;
                    }
                } else {
                    // Column is still too large, so let's count it
                    num_overages += 1;
                    if i != (headers_len - 1) {
                        overage_separator_sum += 3;
                    }
                    if i == 0 {
                        overage_separator_sum += 1;
                    }
                }
            }
        }

        ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        }
    }

    fn max_width(&self, termwidth: usize) -> usize {
        let ColumnSpace {
            num_overages,
            underage_sum,
            overage_separator_sum,
        } = self;

        if *num_overages > 0 {
            (termwidth - 1 - *underage_sum - *overage_separator_sum) / *num_overages
        } else {
            99999
        }
    }
}

fn wrap_cells(
    mut headers: Vec<String>,
    mut entries: Entries,
    max_per_column: Vec<usize>,
    max_naive_column_width: usize,
    max_column_width: usize,
) -> TableView {
    for head in 0..headers.len() {
        if max_per_column[head] > max_naive_column_width {
            headers[head] = fill(&headers[head], max_column_width);

            for entry in entries.iter_mut() {
                entry[head].0 = fill(&entry[head].0, max_column_width);
            }
        }
    }

    TableView { headers, entries }
}

impl RenderView for TableView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let mut table = Table::new();

        let mut config = crate::data::config::config(Tag::unknown())?;
        let header_align = config.get("header_align").map_or(Alignment::LEFT, |a| {
            a.as_string()
                .map_or(Alignment::LEFT, |a| match a.to_lowercase().as_str() {
                    "center" | "c" => Alignment::CENTER,
                    "right" | "r" => Alignment::RIGHT,
                    _ => Alignment::LEFT,
                })
        });

        let header_color = config.get("header_color").map_or(color::GREEN, |c| {
            c.as_string().map_or(color::GREEN, |c| {
                str_to_color(c.to_lowercase()).unwrap_or(color::GREEN)
            })
        });

        let header_style =
            config
                .remove("header_style")
                .map_or(vec![Attr::Bold], |y| match y.value {
                    UntaggedValue::Table(t) => to_style_vec(t),
                    UntaggedValue::Primitive(p) => vec![p
                        .into_string(Span::unknown())
                        .map_or(Attr::Bold, |s| str_to_style(s).unwrap_or(Attr::Bold))],
                    _ => vec![Attr::Bold],
                });

        let table_mode = if let Some(s) = config.get("table_mode") {
            match s.as_string() {
                Ok(typ) if typ == "light" => TableMode::Light,
                _ => TableMode::Normal,
            }
        } else {
            TableMode::Normal
        };

        match table_mode {
            TableMode::Light => {
                table.set_format(
                    FormatBuilder::new()
                        .separator(LinePosition::Title, LineSeparator::new('─', '─', ' ', ' '))
                        .padding(1, 1)
                        .build(),
                );
            }
            _ => {
                table.set_format(
                    FormatBuilder::new()
                        .column_separator('│')
                        .separator(LinePosition::Top, LineSeparator::new('─', '┬', ' ', ' '))
                        .separator(LinePosition::Title, LineSeparator::new('─', '┼', ' ', ' '))
                        .separator(LinePosition::Bottom, LineSeparator::new('─', '┴', ' ', ' '))
                        .padding(1, 1)
                        .build(),
                );
            }
        }

        let skip_headers = (self.headers.len() == 2 && self.headers[1] == "")
            || (self.headers.len() == 1 && self.headers[0] == "");

        let header: Vec<Cell> = self
            .headers
            .iter()
            .map(|h| {
                let mut c = Cell::new_align(h, header_align)
                    .with_style(Attr::ForegroundColor(header_color));
                for &s in &header_style {
                    c.style(s);
                }
                c
            })
            .collect();

        if !skip_headers {
            table.set_titles(Row::new(header));
        }

        for row in &self.entries {
            table.add_row(Row::new(
                row.iter()
                    .map(|(v, s)| Cell::new(v).style_spec(s))
                    .collect(),
            ));
        }

        table.print_term(&mut *host.out_terminal().ok_or_else(|| ShellError::untagged_runtime_error("Could not open terminal for output"))?)
            .map_err(|_| ShellError::untagged_runtime_error("Internal error: could not print to terminal (for unix systems check to make sure TERM is set)"))?;

        Ok(())
    }
}

fn str_to_color(s: String) -> Option<color::Color> {
    match s.as_str() {
        "g" | "green" => Some(color::GREEN),
        "r" | "red" => Some(color::RED),
        "u" | "blue" => Some(color::BLUE),
        "b" | "black" => Some(color::BLACK),
        "y" | "yellow" => Some(color::YELLOW),
        "m" | "magenta" => Some(color::MAGENTA),
        "c" | "cyan" => Some(color::CYAN),
        "w" | "white" => Some(color::WHITE),
        "bg" | "bright green" => Some(color::BRIGHT_GREEN),
        "br" | "bright red" => Some(color::BRIGHT_RED),
        "bu" | "bright blue" => Some(color::BRIGHT_BLUE),
        "by" | "bright yellow" => Some(color::BRIGHT_YELLOW),
        "bm" | "bright magenta" => Some(color::BRIGHT_MAGENTA),
        "bc" | "bright cyan" => Some(color::BRIGHT_CYAN),
        "bw" | "bright white" => Some(color::BRIGHT_WHITE),
        _ => None,
    }
}

fn to_style_vec(a: Vec<Value>) -> Vec<Attr> {
    let mut v: Vec<Attr> = Vec::new();
    for t in a {
        if let Ok(s) = t.as_string() {
            if let Some(r) = str_to_style(s) {
                v.push(r);
            }
        }
    }
    v
}

fn str_to_style(s: String) -> Option<Attr> {
    match s.as_str() {
        "b" | "bold" => Some(Attr::Bold),
        "i" | "italic" | "italics" => Some(Attr::Italic(true)),
        "u" | "underline" | "underlined" => Some(Attr::Underline(true)),
        _ => None,
    }
}
