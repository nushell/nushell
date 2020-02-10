use nu_plugin::serve_plugin;
use nu_plugin_match::Match;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    serve_plugin(&mut Match::new()?);
    Ok(())
}
