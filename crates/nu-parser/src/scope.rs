use nu_source::Spanned;
use std::{collections::HashMap, fmt::Debug};

pub trait CommandScope: Debug {
    fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature>;

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>>;
    fn add_alias(&mut self, name: &str, replacement: Vec<Spanned<String>>);

    fn has_signature(&self, name: &str) -> bool;
}

impl CommandScope for Scope {
    fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature> {
        self.get_signature(name)
    }

    fn has_signature(&self, name: &str) -> bool {
        self.get_signature(name).is_some()
    }

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>> {
        self.get_alias(name)
    }

    fn add_alias(&mut self, name: &str, replacement: Vec<Spanned<String>>) {
        self.aliases.insert(name.to_string(), replacement);
    }
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<Box<dyn CommandScope>>,
    pub commands: HashMap<String, nu_protocol::Signature>,
    pub aliases: HashMap<String, Vec<Spanned<String>>>,
}

impl Scope {
    pub fn new(parent: Option<Box<dyn CommandScope>>) -> Scope {
        Scope {
            parent,
            commands: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
    pub fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature> {
        if let Some(x) = self.commands.get(name) {
            Some(x.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_signature(name)
        } else {
            None
        }
    }

    pub fn has_signature(&self, name: &str) -> bool {
        self.get_signature(name).is_some()
    }

    pub fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>> {
        if let Some(x) = self.aliases.get(name) {
            Some(x.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_alias(name)
        } else {
            None
        }
    }
}
