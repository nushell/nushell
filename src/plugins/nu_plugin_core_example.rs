use nu_plugin::serve_plugin;
use nu_plugin_example::Example;

fn main() {
    serve_plugin(&mut Example {})
}
