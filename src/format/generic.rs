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
                // let mut list: Vec<String> = vec![];
                // for item in l {
                //     match item {
                //         Value::Primitive(p) => list.push(p.format()),
                //         Value::List(l) => list.push(format!("{:?}", l)),
                //         Value::Object(o) => {
                //             let view = o.to_entries_view();
                //             let out = view.render_view(host);
                //             list.extend(out);
                //         }
                //     }
                //     list.push("\n".to_string());
                // }
                // list
            }

            o @ Value::Object(_) => {
                let view = EntriesView::from_value(o);
                view.render_view(host)?;
                Ok(())
            }

            Value::Operation(o) => {
                host.stdout(&format!(
                    "Unexpectedly trying to print an operation: {:?}",
                    o
                ));
                Ok(())
            }

            Value::Error(e) => {
                host.stdout(&format!("{:?}", e));
                Ok(())
            }
        }
    }
}
