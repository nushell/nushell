use nu_plugin::serve_plugin;
use nu_plugin_charts::Chart;

fn main() {
    serve_plugin(&mut Chart::new());
}
