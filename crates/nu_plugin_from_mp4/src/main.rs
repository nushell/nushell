use nu_plugin::serve_plugin;
use nu_plugin_from_mp4::FromMp4;

fn main() {
    serve_plugin(&mut FromMp4::new())
}
