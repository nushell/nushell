use nu_plugin::serve_plugin;
use nu_plugin_selector::Selector;

fn main() {
    serve_plugin(&mut Selector::new());
}
