use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin_query::Query;

fn main() {
    serve_plugin(&mut Query {}, JsonSerializer {})
}
