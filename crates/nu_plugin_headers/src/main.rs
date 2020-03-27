use nu_plugin::serve_plugin;
use nu_plugin_headers::Headers;

//https://github.com/nushell/nushell/issues/1486
fn main() {
    serve_plugin(&mut Headers::new())
}
