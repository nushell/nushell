use nu_plugin::{JsonSerializer, serve_plugin};
use nu_plugin_query::Query;

fn main() {
    serve_plugin(&Query {}, JsonSerializer {})
}
