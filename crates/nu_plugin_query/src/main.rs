use nu_plugin::{serve_plugin, EncodingType};
use nu_plugin_query::Query;

fn main() {
    serve_plugin(&mut Query {}, EncodingType::Json)
}
