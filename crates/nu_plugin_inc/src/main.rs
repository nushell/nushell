use nu_plugin::{serve_plugin, CapnpSerializer};
use nu_plugin_inc::Inc;

fn main() {
    serve_plugin(&mut Inc::new(), CapnpSerializer {})
}
