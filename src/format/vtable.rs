use crate::format::{RenderView, consts};
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

use prettytable::{color, Attr, Cell, Row, Table};

#[derive(new)]
pub struct VTableView {
    entries: Vec<Vec<String>>,
}

impl VTableView {
    pub fn from_list(values: &[Tagged<Value>]) -> Option<VTableView> {
        if values.len() == 0 {
            return None;
        }

        let item = &values[0];
        let headers = item.data_descriptors();

        if headers.len() == 0 {
            return None;
        }

        let mut entries = vec![];

        for header in headers {
            let mut row = vec![];

            row.push(header.clone());
            for value in values {
                row.push(value.get_data(&header).borrow().format_leaf(Some(&header)));
            }
            entries.push(row);
        }

        Some(VTableView { entries })
    }
}

impl RenderView for VTableView {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        if self.entries.len() == 0 {
            return Ok(());
        }

        let mut table = Table::new();
        table.set_format(*consts::TABLE_FORMAT);

        for row in &self.entries {
            table.add_row(Row::new(
                row.iter()
                    .enumerate()
                    .map(|(idx, h)| {
                        if idx == 0 {
                            Cell::new(h)
                                .with_style(Attr::ForegroundColor(color::GREEN))
                                .with_style(Attr::Bold)
                        } else {
                            Cell::new(h)
                        }
                    })
                    .collect(),
            ));
        }

        table.print_term(&mut *host.out_terminal()).unwrap();

        Ok(())
    }
}
