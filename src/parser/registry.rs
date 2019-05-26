use indexmap::IndexMap;

#[allow(unused)]
pub enum CommandType {
    Switch,
    Single,
    Array,
}

#[allow(unused)]
pub struct CommandConfig {
    crate name: String,
    crate mandatory_positional: Vec<String>,
    crate optional_positional: Vec<String>,
    crate rest_positional: bool,
    crate named: IndexMap<String, CommandType>,
}

pub trait CommandRegistry {
    fn get(&self, name: &str) -> CommandConfig;
}
