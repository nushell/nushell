use crate::format::RenderView;
use crate::prelude::*;
use derive_new::new;
use ptree::item::StringItem;
use ptree::output::print_tree_with;
use ptree::print_config::PrintConfig;
use ptree::style::{Color, Style};
use ptree::TreeBuilder;

// An entries list is printed like this:
//
// name         : ...
// name2        : ...
// another_name : ...
#[derive(new)]
pub struct TreeView {
    //entries: Vec<(crate::object::DescriptorName, Value)>,
    tree: StringItem,
}

impl TreeView {
    fn from_value_helper(value: &Value, mut builder: &mut TreeBuilder) {
        match value {
            Value::Primitive(p) => {
                let _ = builder.add_empty_child(p.format(None));
            }
            Value::Object(o) => {
                for (k, v) in o.entries.iter() {
                    builder = builder.begin_child(k.name.display().to_string());
                    Self::from_value_helper(v, builder);
                    builder = builder.end_child();
                }
            }
            Value::List(l) => {
                for elem in l.iter() {
                    Self::from_value_helper(elem, builder);
                }
            }
            Value::Block(_) => {}
            Value::Error(_) => {}
            Value::Filesystem => {}
        }
    }
    crate fn from_value(value: &Value) -> TreeView {
        let descs = value.data_descriptors();

        let mut tree = TreeBuilder::new("".to_string());
        let mut builder = &mut tree;

        for desc in descs {
            let value = value.get_data(&desc);
            builder = builder.begin_child(desc.name.display().to_string());
            Self::from_value_helper(value.borrow(), &mut builder);
            builder = builder.end_child();
            //entries.push((desc.name.clone(), value.borrow().copy()))
        }

        TreeView::new(builder.build())
    }
}

impl RenderView for TreeView {
    fn render_view(&self, _host: &mut dyn Host) -> Result<(), ShellError> {
        // Set up the print configuration
        let config = {
            let mut config = PrintConfig::from_env();
            config.branch = Style {
                foreground: Some(Color::Green),
                dimmed: true,
                ..Style::default()
            };
            config.leaf = Style {
                bold: true,
                ..Style::default()
            };
            //config.characters = UTF_CHARS_BOLD.into();
            config.indent = 4;
            config
        };

        // Print out the tree using custom formatting
        print_tree_with(&self.tree, &config)?;

        Ok(())
    }
}
