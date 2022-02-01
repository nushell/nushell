use nu_plugin::{serve_plugin, CapnpSerializer};
use nu_plugin_query::Query;

fn main() {
    serve_plugin(&mut Query {}, CapnpSerializer {})
}
