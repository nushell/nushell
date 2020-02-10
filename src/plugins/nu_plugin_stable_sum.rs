use nu_plugin::serve_plugin;
use nu_plugin_sum::Sum;

fn main() {
    serve_plugin(&mut Sum::new());
}
