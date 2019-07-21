use crate::format::RenderView;
use crate::object::Value;
use crate::prelude::*;
use ansi_term::Color;
use derive_new::new;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};

use prettytable::{color, Attr, Cell, Row, Table};

#[derive(new)]
pub struct TableView {
    headers: Vec<String>,
    entries: Vec<Vec<String>>,
}

impl TableView {
    fn merge_descriptors(values: &[Spanned<Value>]) -> Vec<String> {
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

    pub fn from_list(values: &[Spanned<Value>]) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let mut headers = TableView::merge_descriptors(values);

        if headers.len() == 0 {
            headers.push("value".to_string());
        }

        let mut entries = vec![];

        for (idx, value) in values.iter().enumerate() {
            let mut row: Vec<String> = headers
                .iter()
                .enumerate()
                .map(|(i, d)| value.get_data(d).borrow().format_leaf(Some(&headers[i])))
                .collect();

            if values.len() > 1 {
                row.insert(0, format!("{}", Color::Black.bold().paint(idx.to_string())));
            }
            entries.push(row);
        }

        if values.len() > 1 {
            headers.insert(0, format!("#"));
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
