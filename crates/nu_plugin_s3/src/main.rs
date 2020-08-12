use nu_plugin::serve_plugin;
use nu_plugin_s3::handler;

fn main() {
    serve_plugin(&mut handler::Handler::new())
}
