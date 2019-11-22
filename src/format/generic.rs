use crate::data::Value;
use crate::format::{EntriesView, RenderView, TableView};
use crate::prelude::*;
use derive_new::new;

// A list is printed one line at a time with an optional separator between groups
#[derive(new)]
pub struct GenericView<'value> {
    value: &'value Value,
}

impl RenderView for GenericView<'_> {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        match self.value {
            Value::Primitive(p) => Ok(host.stdout(&p.format(None))),
            Value::Table(l) => {
                let view = TableView::from_list(l, 0);

                if let Some(view) = view {
                    view.render_view(host)?;
                }

                Ok(())
            }

            o @ Value::Row(_) => {
                let view = EntriesView::from_value(o);
                view.render_view(host)?;
                Ok(())
            }

            b @ Value::Block(_) => {
                let printed = b.format_leaf().plain_string(host.width());
                let view = EntriesView::from_value(&Value::string(printed));
                view.render_view(host)?;
                Ok(())
            }

            Value::Error(e) => Err(e.clone()),
        }
    }
}
