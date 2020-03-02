use crate::data::value::format_leaf;
use crate::format::{EntriesView, RenderView, TableView};
use crate::prelude::*;
use derive_new::new;
use nu_errors::ShellError;
use nu_protocol::{format_primitive, UntaggedValue, Value};

// A list is printed one line at a time with an optional separator between groups
#[derive(new)]
pub struct GenericView<'value> {
    value: &'value Value,
}

impl RenderView for GenericView<'_> {
    fn render_view(&self, host: &mut dyn Host) -> Result<(), ShellError> {
        let tag = &self.value.tag;
        match &self.value.value {
            UntaggedValue::Primitive(p) => {
                host.stdout(&format_primitive(p, None));
                Ok(())
            }
            UntaggedValue::Table(l) => {
                let view = TableView::from_list(l, 0);

                if let Some(view) = view {
                    view.render_view(host)?;
                }

                Ok(())
            }

            o @ UntaggedValue::Row(_) => {
                let view = EntriesView::from_value(&o.clone().into_value(tag));
                view.render_view(host)?;
                Ok(())
            }

            b @ UntaggedValue::Block(_) => {
                let printed = format_leaf(b).plain_string(host.width());
                let view = EntriesView::from_value(&UntaggedValue::string(printed).into_value(tag));
                view.render_view(host)?;
                Ok(())
            }

            UntaggedValue::Error(e) => Err(e.clone()),
        }
    }
}
