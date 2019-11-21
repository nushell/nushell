use derive_new::new;
use nu::{serve_plugin, CallInfo, Plugin, ShellError, Signature, Tagged, Value};
use ptree::item::StringItem;
use ptree::output::print_tree_with;
use ptree::print_config::PrintConfig;
use ptree::style::{Color, Style};
use ptree::TreeBuilder;

#[derive(new)]
pub struct TreeView {
    tree: StringItem,
}

impl TreeView {
    fn from_value_helper(value: &Value, mut builder: &mut TreeBuilder) {
        match value {
            UntaggedValue::Primitive(p) => {
                let _ = builder.add_empty_child(p.format(None));
            }
            UntaggedValue::Row(o) => {
                for (k, v) in o.entries.iter() {
                    builder = builder.begin_child(k.clone());
                    Self::from_value_helper(v, builder);
                    builder = builder.end_child();
                }
            }
            UntaggedValue::Table(l) => {
                for elem in l.iter() {
                    Self::from_value_helper(elem, builder);
                }
            }
            UntaggedValue::Block(_) => {}
            UntaggedValue::Binary(_) => {}
        }
    }

    fn from_value(value: &Value) -> TreeView {
        let descs = value.data_descriptors();

        let mut tree = TreeBuilder::new("".to_string());
        let mut builder = &mut tree;

        for desc in descs {
            let value = value.get_data(&desc);
            builder = builder.begin_child(desc.clone());
            Self::from_value_helper(value.borrow(), &mut builder);
            builder = builder.end_child();
            //entries.push((desc.name.clone(), value.borrow().copy()))
        }

        TreeView::new(builder.build())
    }

    fn render_view(&self) -> Result<(), ShellError> {
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
            config.indent = 4;
            config
        };

        // Print out the tree using custom formatting
        print_tree_with(&self.tree, &config)?;

        Ok(())
    }
}

struct TreeViewer;

impl Plugin for TreeViewer {
    fn config(&mut self) -> Result<Signature, ShellError> {
        Ok(Signature::build("tree").desc("View the contents of the pipeline as a tree."))
    }

    fn sink(&mut self, _call_info: CallInfo, input: Vec<Value>) {
        if input.len() > 0 {
            for i in input.iter() {
                let view = TreeView::from_value(&i);
                let _ = view.render_view();
            }
        }
    }
}

fn main() {
    serve_plugin(&mut TreeViewer);
}
