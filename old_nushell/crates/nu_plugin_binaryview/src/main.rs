use nu_plugin::serve_plugin;
use nu_plugin_binaryview::BinaryView;

fn main() {
    serve_plugin(&mut BinaryView::new())
}
