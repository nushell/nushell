use nu_plugin::serve_plugin;
use nu_plugin_chart::Chart;

fn main() {
    serve_plugin(&mut Chart::new());
}
