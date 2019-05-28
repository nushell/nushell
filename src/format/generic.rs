use crate::format::{EntriesView, RenderView, TableView};
use crate::object::Value;
use crate::prelude::*;
use derive_new::new;

// A list is printed one line at a time with an optional separator between groups
#[derive(new)]
pub struct GenericView<'value> {
    value: &'value Value,
}

impl RenderView for GenericView<'value> {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        match self.value {
            Value::Primitive(p) => Ok(host.stdout(&p.format(None))),
            Value::List(l) => {
                let view = TableView::from_list(l);

                if let Some(view) = view {
                    view.render_view(host)?;
                }

                Ok(())
            }

            o @ Value::Object(_) => {
                let view = EntriesView::from_value(o);
                view.render_view(host)?;
                Ok(())
            }

            b @ Value::Block(_) => {
                let printed = b.format_leaf(None);
                let view = EntriesView::from_value(&Value::string(&printed));
                view.render_view(host)?;
                Ok(())
            }

            Value::Error(e) => {
                // println!("ERROR: {:?}", e);
                host.stdout(&format!("{:?}", e));
                Ok(())
            }
        }
    }
}
