use nu_plugin::serve_plugin;
use nu_plugin_average::Average;

fn main() {
    serve_plugin(&mut Average::new());
}
