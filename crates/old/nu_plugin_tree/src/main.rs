use nu_plugin::serve_plugin;
use nu_plugin_tree::TreeViewer;

fn main() {
    serve_plugin(&mut TreeViewer);
}
