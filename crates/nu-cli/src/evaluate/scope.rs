use crate::commands::Command;
use crate::prelude::*;
use nu_parser::ParserScope;
use nu_protocol::Value;
use nu_source::Spanned;

/// An evaluation scope. Scopes map variable names to Values and aid in evaluating blocks and expressions.
#[derive(Debug, Clone)]
pub struct Scope {
    vars: IndexMap<String, Value>,
    env: IndexMap<String, String>,
    commands: IndexMap<String, Command>,
    aliases: IndexMap<String, Vec<Spanned<String>>>,
    parent: Option<Arc<Scope>>,
}

impl Scope {
    pub fn has_command(&self, name: &str) -> bool {
        self.get_command(name).is_some()
    }

    pub fn get_command_names(&self) -> Vec<String> {
        let mut parent_command_names = if let Some(parent) = &self.parent {
            parent.get_command_names()
        } else {
            vec![]
        };

        let mut command_names: Vec<String> = self.commands.keys().map(|x| x.to_string()).collect();
        parent_command_names.append(&mut command_names);
        command_names.dedup();
        command_names.sort();

        command_names
    }

    pub fn add_command(&mut self, name: String, command: Command) {
        self.commands.insert(name, command);
    }

    pub fn get_command(&self, name: &str) -> Option<Command> {
        if let Some(command) = self.commands.get(name) {
            Some(command.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_command(name)
        } else {
            None
        }
    }

    pub fn expect_command(&self, name: &str) -> Result<Command, ShellError> {
        if let Some(c) = self.get_command(name) {
            Ok(c)
        } else {
            Err(ShellError::untagged_runtime_error(format!(
                "Missing command '{}'",
                name
            )))
        }
    }
}

impl ParserScope for Scope {
    fn get_signature(&self, name: &str) -> Option<nu_protocol::Signature> {
        self.get_command(name).map(|x| x.signature())
    }

    fn has_signature(&self, name: &str) -> bool {
        self.get_command(name).is_some()
    }

    fn get_alias(&self, name: &str) -> Option<Vec<Spanned<String>>> {
        if let Some(x) = self.aliases.get(name) {
            Some(x.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_alias(name)
        } else {
            None
        }
    }

    fn add_alias(&mut self, name: &str, replacement: Vec<Spanned<String>>) {
        self.aliases.insert(name.to_string(), replacement);
    }

    fn enter_scope(&self) -> Arc<dyn ParserScope> {
        Arc::new(Scope {
            aliases: IndexMap::new(),
            commands: IndexMap::new(),
            env: IndexMap::new(),
            vars: IndexMap::new(),
            parent: Some(Arc::new(self.clone())),
        })
    }
}

impl Scope {
    pub fn vars(&self) -> IndexMap<String, Value> {
        //FIXME: should this be an interator?

        let mut output = IndexMap::new();

        for v in &self.vars {
            output.insert(v.0.clone(), v.1.clone());
        }

        if let Some(parent) = &self.parent {
            for v in parent.vars() {
                if !output.contains_key(&v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output
    }

    pub fn env(&self) -> IndexMap<String, String> {
        //FIXME: should this be an interator?

        let mut output = IndexMap::new();

        for v in &self.env {
            output.insert(v.0.clone(), v.1.clone());
        }

        if let Some(parent) = &self.parent {
            for v in parent.env() {
                if !output.contains_key(&v.0) {
                    output.insert(v.0.clone(), v.1.clone());
                }
            }
        }

        output
    }

    pub fn var(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.vars().get(name) {
            Some(value.clone())
        } else {
            None
        }
    }

    pub fn append_var(this: Arc<Self>, name: impl Into<String>, value: Value) -> Arc<Scope> {
        let mut vars = IndexMap::new();
        vars.insert(name.into(), value);
        Arc::new(Scope {
            vars,
            env: IndexMap::new(),
            commands: IndexMap::new(),
            aliases: IndexMap::new(),
            parent: Some(this),
        })
    }

    pub fn append_vars(this: Arc<Self>, vars: IndexMap<String, Value>) -> Arc<Scope> {
        Arc::new(Scope {
            vars,
            env: IndexMap::new(),
            commands: IndexMap::new(),
            aliases: IndexMap::new(),
            parent: Some(this),
        })
    }

    pub fn append_env(this: Arc<Self>, env: IndexMap<String, String>) -> Arc<Scope> {
        Arc::new(Scope {
            vars: IndexMap::new(),
            env,
            commands: IndexMap::new(),
            aliases: IndexMap::new(),
            parent: Some(this),
        })
    }

    /// Create an empty scope
    pub fn create() -> Arc<Scope> {
        Arc::new(Scope {
            vars: IndexMap::new(),
            env: IndexMap::new(),
            commands: IndexMap::new(),
            aliases: IndexMap::new(),
            parent: None,
        })
    }
}
