use nu_plugin::serve_plugin;
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&mut Inc::new())
}
