use crate::data::Value;
use crate::format::RenderView;
use crate::prelude::*;
use crate::traits::{DebugDocBuilder, DebugDocBuilder as b, PrettyDebug};
use derive_new::new;
use textwrap::fill;

use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
use prettytable::{color, Attr, Cell, Row, Table};

#[derive(Debug, new)]
pub struct TableView {
    // List of header cell values:
    headers: Vec<String>,

    // List of rows of cells, each containing value and prettytable style-string:
    entries: Vec<Vec<(String, &'static str)>>,
}

enum TableMode {
    Light,
    Normal,
}

impl TableView {
    fn merge_descriptors(values: &[Tagged<Value>]) -> Vec<String> {
        let mut ret: Vec<String> = vec![];
        let value_column = "<value>".to_string();
        for value in values {
            let descs = value.data_descriptors();

            if descs.len() == 0 {
                if !ret.contains(&value_column) {
                    ret.push("<value>".to_string());
                }
            } else {
                for desc in value.data_descriptors() {
                    if !ret.contains(&desc) {
                        ret.push(desc);
                    }
                }
            }
        }
        ret
    }

    pub fn from_list(values: &[Tagged<Value>], starting_idx: usize) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let mut headers = TableView::merge_descriptors(values);

        if headers.len() == 0 {
            headers.push("<value>".to_string());
        }

        let mut entries = vec![];

        for (idx, value) in values.iter().enumerate() {
            let mut row: Vec<(DebugDocBuilder, &'static str)> = match value {
                Tagged {
                    item: Value::Row(..),
                    ..
                } => headers
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let data = value.get_data(d);
                        return (
                            data.borrow().format_for_column(&headers[i]),
                            data.borrow().style_leaf(),
                        );
                    })
                    .collect(),
                x => vec![(x.format_leaf(), x.style_leaf())],
            };

            if values.len() > 1 {
                // Indices are black, bold, right-aligned:
                row.insert(
                    0,
                    (
                        b::primitive(format!("{}", (starting_idx + idx).to_string())),
                        "Fdbr",
                    ),
                );
            }

            entries.push(row);
        }

        let mut max_per_column = vec![];

        if values.len() > 1 {
            headers.insert(0, format!("#"));
        }

        // Different platforms want different amounts of buffer, not sure why
        let termwidth = std::cmp::max(textwrap::termwidth(), 20);

        for head in 0..headers.len() {
            let mut current_col_max = 0;
            for row in 0..values.len() {
                let value_length = entries[row][head].0.plain_string(termwidth).chars().count();
                if value_length > current_col_max {
                    current_col_max = value_length;
                }
            }

            max_per_column.push(std::cmp::max(
                current_col_max,
                headers[head].chars().count(),
            ));
        }

        // Make sure we have enough space for the columns we have
        let max_num_of_columns = termwidth / 10;

        // If we have too many columns, truncate the table
        if max_num_of_columns < headers.len() {
            headers.truncate(max_num_of_columns);
            for row in 0..entries.len() {
                entries[row].truncate(max_num_of_columns);
            }

            headers.push("...".to_string());
            for row in 0..entries.len() {
                entries[row].push((b::description("..."), "c")); // ellipsis is centred
            }
        }

        // Measure how big our columns need to be (accounting for separators also)
        let max_naive_column_width = (termwidth - 3 * (headers.len() - 1)) / headers.len();

        // Measure how much space we have once we subtract off the columns who are small enough
        let mut num_overages = 0;
        let mut underage_sum = 0;
        let mut overage_separator_sum = 0;
        for idx in 0..headers.len() {
            if max_per_column[idx] > max_naive_column_width {
                num_overages += 1;
                if idx != (headers.len() - 1) {
                    overage_separator_sum += 3;
                }
                if idx == 0 {
                    overage_separator_sum += 1;
                }
            } else {
                underage_sum += max_per_column[idx];
                // if column isn't last, add 3 for its separator
                if idx != (headers.len() - 1) {
                    underage_sum += 3;
                }
                if idx == 0 {
                    underage_sum += 1;
                }
            }
        }

        // This gives us the max column width
        let max_column_width = if num_overages > 0 {
            (termwidth - 1 - underage_sum - overage_separator_sum) / num_overages
        } else {
            99999
        };

        // This width isn't quite right, as we're rounding off some of our space
        num_overages = 0;
        overage_separator_sum = 0;
        for idx in 0..headers.len() {
            if max_per_column[idx] > max_naive_column_width {
                if max_per_column[idx] <= max_column_width {
                    underage_sum += max_per_column[idx];
                    // if column isn't last, add 3 for its separator
                    if idx != (headers.len() - 1) {
                        underage_sum += 3;
                    }
                    if idx == 0 {
                        underage_sum += 1;
                    }
                } else {
                    // Column is still too large, so let's count it
                    num_overages += 1;
                    if idx != (headers.len() - 1) {
                        overage_separator_sum += 3;
                    }
                    if idx == 0 {
                        overage_separator_sum += 1;
                    }
                }
            }
        }
        // This should give us the final max column width
        let max_column_width = if num_overages > 0 {
            (termwidth - 1 - underage_sum - overage_separator_sum) / num_overages
        } else {
            99999
        };

        let mut out_entries: Vec<Vec<(String, &'static str)>> = Vec::with_capacity(entries.len());

        for row in entries.iter() {
            let mut out_row = Vec::with_capacity(row.len());

            for _ in row {
                out_row.push((String::new(), "Fdbr"));
            }

            out_entries.push(out_row);
        }

        // Wrap cells as needed
        for head in 0..headers.len() {
            if max_per_column[head] > max_naive_column_width {
                if true_width(&headers[head]) > max_column_width {
                    headers[head] = fill(&headers[head], max_column_width);
                }

                for (i, row) in entries.iter().enumerate() {
                    let column = &row[head].0;
                    let column = column.plain_string(max_column_width);

                    out_entries[i][head] = (column, row[head].1);
                }
            } else {
                for (i, row) in entries.iter().enumerate() {
                    let column = &row[head].0;
                    let column = column.plain_string(max_column_width);

                    out_entries[i][head] = (column, row[head].1);
                }
            }
        }

        Some(TableView {
            headers,
            entries: out_entries,
        })
    }
}

fn true_width(string: &str) -> usize {
    let stripped = console::strip_ansi_codes(string);

    stripped.lines().map(|line| line.len()).max().unwrap_or(0)
}

impl RenderView for TableView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.len() == 0 {
            return Ok(());
        }

        let mut table = Table::new();

        let table_mode = crate::data::config::config(Tag::unknown())?
            .get("table_mode")
            .map(|s| match s.as_string().unwrap().as_ref() {
                "light" => TableMode::Light,
                _ => TableMode::Normal,
            })
            .unwrap_or(TableMode::Normal);

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
                        .separator(LinePosition::Top, LineSeparator::new('━', '┯', ' ', ' '))
                        .separator(LinePosition::Title, LineSeparator::new('─', '┼', ' ', ' '))
                        .separator(LinePosition::Bottom, LineSeparator::new('━', '┷', ' ', ' '))
                        .padding(1, 1)
                        .build(),
                );
            }
        }

        let header: Vec<Cell> = self
            .headers
            .iter()
            .map(|h| {
                Cell::new(h)
                    .with_style(Attr::ForegroundColor(color::GREEN))
                    .with_style(Attr::Bold)
            })
            .collect();

        table.set_titles(Row::new(header));

        for row in &self.entries {
            table.add_row(Row::new(
                row.iter()
                    .map(|(v, s)| Cell::new(v).style_spec(s))
                    .collect(),
            ));
        }

        table.print_term(&mut *host.out_terminal()).unwrap();

        Ok(())
    }
}
