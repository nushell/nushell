use nu_plugin::serve_plugin;
use nu_plugin_query_json::QueryJson;

fn main() {
    serve_plugin(&mut QueryJson::new());
}
