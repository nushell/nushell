use crate::format::RenderView;
use crate::object::{DataDescriptor, Value};
use crate::prelude::*;
use derive_new::new;
use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};

use prettytable::{color, Attr, Cell, Row, Table};

#[derive(new)]
pub struct TableView {
    headers: Vec<DataDescriptor>,
    entries: Vec<Vec<String>>,
}

impl TableView {
    pub fn from_list(values: &[Value]) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let item = &values[0];
        let headers = item.data_descriptors();

        if headers.len() == 0 {
            return None;
        }

        let mut entries = vec![];

        for value in values {
            let row = headers
                .iter()
                .enumerate()
                .map(|(i, d)| value.get_data(d).borrow().format_leaf(Some(&headers[i])))
                .collect();

            entries.push(row);
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
                Cell::new(h.display_header())
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
