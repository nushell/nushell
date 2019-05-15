use crate::format::{EntriesView, RenderView, TableView};
use crate::object::Value;
use crate::Host;
use derive_new::new;

// A list is printed one line at a time with an optional separator between groups
#[derive(new)]
pub struct GenericView<'value> {
    value: &'value Value,
}

impl RenderView for GenericView<'value> {
    fn render_view(&self, host: &dyn Host) -> Vec<String> {
        match self.value {
            Value::Primitive(p) => vec![p.format(None)],
            Value::List(l) => {
                let view = TableView::from_list(l);

                if let Some(view) = view {
                    view.render_view(host)
                } else {
                    vec![]
                }
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
                let out = view.render_view(host);
                out
            }

            Value::Error(e) => vec![format!("{}", e)],
        }
    }
}
