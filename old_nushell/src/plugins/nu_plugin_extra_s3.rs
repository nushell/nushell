use nu_plugin::serve_plugin;
use nu_plugin_s3::Handler;

fn main() {
    serve_plugin(&mut Handler::new());
}
