use nu_plugin::{serve_plugin, CapnpSerializer};
use nu_plugin_gstat::GStat;

fn main() {
    serve_plugin(&mut GStat::new(), CapnpSerializer {})
}
