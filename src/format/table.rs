use crate::format::RenderView;
use crate::object::Value;
use crate::prelude::*;
use ansi_term::Color;
use derive_new::new;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};

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

    pub fn from_list(values: &[Tagged<Value>], full: bool) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let max_row: usize = termsize::get().map(|x| x.cols).unwrap() as usize;

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

        if values.len() > 1 {
            headers.insert(0, format!("#"));
        }

        // Trim if the row is too long
        let mut max_up_to_now = 0;

        let num_headers = headers.len();
        let mut had_to_trim_off_rows = false;

        for i in 0..num_headers {
            let mut max_entry = 0;

            if (max_up_to_now + 8) >= max_row && !full {
                had_to_trim_off_rows = true;
                headers.pop();
                for j in 0..entries.len() {
                    entries[j].pop();
                }
            } else {
                if i == (num_headers - 1) {
                    let amount = max_row - std::cmp::min(max_row, max_up_to_now);
                    if headers[i].len() > amount && !full {
                        headers[i] = headers[i].chars().take(amount).collect::<String>();
                        headers[i].push_str("...");
                    }
                } else {
                    if headers[i].len() > (max_row / num_headers) && !full {
                        headers[i] = headers[i]
                            .chars()
                            .take(std::cmp::max(max_row / headers.len(), 5))
                            .collect::<String>();
                        headers[i].push_str("...");
                    }
                }

                if headers[i].len() > max_entry {
                    max_entry = headers[i].len();
                }

                for j in 0..entries.len() {
                    if i == (num_headers - 1) {
                        let amount = max_row - std::cmp::min(max_row, max_up_to_now);
                        if entries[j][i].len() > amount && !full {
                            entries[j][i] = entries[j][i].chars().take(amount).collect::<String>();
                            entries[j][i].push_str("...");
                        }
                    } else {
                        if entries[j][i].len() > (max_row / num_headers) && !full {
                            entries[j][i] = entries[j][i]
                                .chars()
                                .take(std::cmp::max(max_row / headers.len(), 5))
                                .collect::<String>();
                            entries[j][i].push_str("...");
                        }
                    }
                    if entries[j][i].len() > max_entry {
                        max_entry = entries[j][i].len();
                    }
                }

                max_up_to_now += max_entry + 3;
            }
        }

        if had_to_trim_off_rows {
            headers.push("...".to_string());
            for j in 0..entries.len() {
                entries[j].push("...".to_string());
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
