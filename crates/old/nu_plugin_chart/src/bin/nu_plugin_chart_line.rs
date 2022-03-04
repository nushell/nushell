use nu_plugin::serve_plugin;
use nu_plugin_chart::ChartLine;

fn main() {
    serve_plugin(&mut ChartLine::new());
}
