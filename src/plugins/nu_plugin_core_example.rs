use nu_plugin::{serve_plugin, CapnpSerializer};
use nu_plugin_example::Example;

fn main() {
    serve_plugin(&mut Example {}, CapnpSerializer {})
}
