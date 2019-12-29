use crate::hir::Expression;
use crate::Flag;
use indexmap::IndexMap;
use log::trace;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Tag),
    AbsentValue,
    Value(Expression),
}

impl PrettyDebugWithSource for NamedValue {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            NamedValue::AbsentSwitch => b::typed("switch", b::description("absent")),
            NamedValue::PresentSwitch(_) => b::typed("switch", b::description("present")),
            NamedValue::AbsentValue => b::description("absent"),
            NamedValue::Value(value) => value.pretty_debug(source),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamedArguments {
    pub named: IndexMap<String, NamedValue>,
}

impl NamedArguments {
    pub fn new() -> NamedArguments {
        Default::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &NamedValue)> {
        self.named.iter()
    }
}

impl NamedArguments {
    pub fn insert_switch(&mut self, name: impl Into<String>, switch: Option<Flag>) {
        let name = name.into();
        trace!("Inserting switch -- {} = {:?}", name, switch);

        match switch {
            None => self.named.insert(name, NamedValue::AbsentSwitch),
            Some(flag) => self.named.insert(
                name,
                NamedValue::PresentSwitch(Tag {
                    span: *flag.name(),
                    anchor: None,
                }),
            ),
        };
    }

    pub fn insert_optional(&mut self, name: impl Into<String>, expr: Option<Expression>) {
        match expr {
            None => self.named.insert(name.into(), NamedValue::AbsentValue),
            Some(expr) => self.named.insert(name.into(), NamedValue::Value(expr)),
        };
    }

    pub fn insert_mandatory(&mut self, name: impl Into<String>, expr: Expression) {
        self.named.insert(name.into(), NamedValue::Value(expr));
    }
}

impl PrettyDebugWithSource for NamedArguments {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "(",
            b::intersperse(
                self.named
                    .iter()
                    .map(|(key, value)| b::key(key) + b::equals() + value.pretty_debug(source)),
                b::space(),
            ),
            ")",
        )
    }
}
