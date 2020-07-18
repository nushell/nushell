use nu_plugin::serve_plugin;
use nu_plugin_from_sqlite::FromSqlite;

fn main() {
    serve_plugin(&mut FromSqlite::new())
}
