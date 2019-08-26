use crate::format::RenderView;
use crate::object::Value;
use crate::prelude::*;
use ansi_term::Color;
use derive_new::new;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};
use textwrap::fill;

use prettytable::{color, Attr, Cell, Row, Table};

#[derive(Debug, new)]
pub struct TableView {
    headers: Vec<String>,
    entries: Vec<Vec<String>>,
}

impl TableView {
    fn merge_descriptors(values: &[Tagged<Value>]) -> Vec<String> {
        let mut ret = vec![];
        for value in values {
            for desc in value.data_descriptors() {
                if !ret.contains(&desc) {
                    ret.push(desc);
                }
            }
        }
        ret
    }

    pub fn from_list(values: &[Tagged<Value>]) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let mut headers = TableView::merge_descriptors(values);

        if headers.len() == 0 {
            headers.push("value".to_string());
        }

        let mut entries = vec![];

        for (idx, value) in values.iter().enumerate() {
            let mut row: Vec<String> = match value {
                Tagged {
                    item: Value::Object(..),
                    ..
                } => headers
                    .iter()
                    .enumerate()
                    .map(|(i, d)| value.get_data(d).borrow().format_leaf(Some(&headers[i])))
                    .collect(),
                x => vec![x.format_leaf(None)],
            };

            if values.len() > 1 {
                row.insert(0, format!("{}", idx.to_string()));
            }
            entries.push(row);
        }

        let mut max_per_column = vec![];

        if values.len() > 1 {
            headers.insert(0, format!("#"));
        }

        for head in 0..headers.len() {
            let mut current_row_max = 0;
            for row in 0..values.len() {
                if head > entries[row].len() && entries[row][head].len() > current_row_max {
                    current_row_max = entries[row][head].len();
                }
            }
            max_per_column.push(std::cmp::max(current_row_max, headers[head].len()));
        }

        // Different platforms want different amounts of buffer, not sure why
        let termwidth = std::cmp::max(textwrap::termwidth(), 20);

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
                entries[row].push("...".to_string());
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

        // Wrap cells as needed
        for head in 0..headers.len() {
            if max_per_column[head] > max_naive_column_width {
                headers[head] = fill(&headers[head], max_column_width);
                for row in 0..entries.len() {
                    entries[row][head] = fill(&entries[row][head], max_column_width);
                }
            }
        }

        // Paint the number column, if it exists
        if entries.len() > 1 {
            for row in 0..entries.len() {
                entries[row][0] =
                    format!("{}", Color::Black.bold().paint(entries[row][0].to_string()));
            }
        }

        Some(TableView { headers, entries })
    }
}

impl RenderView for TableView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.len() == 0 {
            return Ok(());
        }

        let mut table = Table::new();

        let fb = FormatBuilder::new()
            .separator(LinePosition::Top, LineSeparator::new('-', '+', ' ', ' '))
            .separator(LinePosition::Bottom, LineSeparator::new('-', '+', ' ', ' '))
            .separator(LinePosition::Title, LineSeparator::new('-', '+', '|', '|'))
            .column_separator('|')
            .padding(1, 1);

        //table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
        table.set_format(fb.build());

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
            table.add_row(Row::new(row.iter().map(|h| Cell::new(h)).collect()));
        }

        table.print_term(&mut *host.out_terminal()).unwrap();

        Ok(())
    }
}
