use crate::format::RenderView;
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;
use prettytable::{color, Attr, Cell, Row, Table};

// An entries list is printed like this:
//
// name         : ...
// name2        : ...
// another_name : ...
#[derive(new)]
pub struct TableView {
    headers: Vec<String>,
    entries: Vec<Vec<String>>,
}

impl TableView {
    pub fn from_list(values: &[Value]) -> Option<TableView> {
        if values.len() == 0 {
            return None;
        }

        let item = &values[0];
        let descs = item.data_descriptors();

        if descs.len() == 0 {
            return None;
        }

        let headers: Vec<String> = descs.iter().map(|d| d.name.display().to_string()).collect();

        let mut entries = vec![];

        for value in values {
            let row = descs
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

        table.set_format(*prettytable::format::consts::FORMAT_NO_COLSEP);

        let header: Vec<Cell> = self
            .headers
            .iter()
            .map(|h| {
                Cell::new(h)
                    .with_style(Attr::ForegroundColor(color::GREEN))
                    .with_style(Attr::Bold)
            })
            .collect();

        table.add_row(Row::new(header));

        for row in &self.entries {
            table.add_row(Row::new(row.iter().map(|h| Cell::new(h)).collect()));
        }

        table.print_term(&mut *host.out_terminal()).unwrap();

        Ok(())
    }
}
