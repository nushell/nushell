use nu_plugin::serve_plugin;
use nu_plugin_post::Post;

fn main() {
    serve_plugin(&mut Post::new());
}
