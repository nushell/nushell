use nu_plugin::serve_plugin;
use nu_plugin_chart::ChartBar;

fn main() {
    serve_plugin(&mut ChartBar::new());
}
