use nu_plugin::serve_plugin;
use nu_plugin_to_sqlite::ToSqlite;

fn main() {
    serve_plugin(&mut ToSqlite::new())
}
