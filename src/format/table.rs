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
                row.insert(0, format!("{}", Color::Black.bold().paint(idx.to_string())));
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
                if entries[row][head].len() > current_row_max {
                    current_row_max = entries[row][head].len();
                }
            }
            max_per_column.push(current_row_max);
        }

        let termwidth = textwrap::termwidth() - 5;

        // Make sure we have enough space for the columns we have
        let max_num_of_columns = termwidth / 7;

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

        // Measure how big our columns need to be
        let max_naive_column_width = termwidth / headers.len();

        let mut num_overages = 0;
        let mut underage_sum = 0;
        for idx in 0..headers.len() {
            if max_per_column[idx] > max_naive_column_width {
                num_overages += 1;
            } else {
                underage_sum += max_per_column[idx];
            }
        }

        // Wrap cells as needed
        for head in 0..headers.len() {
            if max_per_column[head] > max_naive_column_width {
                let max_column_width = (termwidth - underage_sum) / num_overages;
                //Account for the separator
                let max_column_width = if max_column_width > 1 {
                    max_column_width - 2
                } else {
                    max_column_width
                };
                println!("{}", max_column_width);
                headers[head] = fill(&headers[head], max_column_width);
                for row in 0..entries.len() {
                    entries[row][head] = fill(&entries[row][head], max_column_width);
                }
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
