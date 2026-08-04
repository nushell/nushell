#[allow(unused, reason = "doesn't matter anymore")]
pub mod deprecated;

pub struct Playground {}

// compatibility
impl Playground {
    pub fn setup<R>(topic: &str, block: impl FnOnce(deprecated::Dirs, &mut deprecated::Playground) -> R) -> R {
        deprecated::Playground::setup(topic, block)
    }
}