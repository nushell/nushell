use crate::parser::hir::Expression;
use crate::parser::{Flag, Span};
use derive_new::new;
use indexmap::IndexMap;
use log::trace;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Span),
    AbsentValue,
    Value(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct NamedArguments {
    #[new(default)]
    crate named: IndexMap<String, NamedValue>,
}

impl NamedArguments {
    pub fn insert_switch(&mut self, name: impl Into<String>, switch: Option<Flag>) {
        let name = name.into();
        trace!("Inserting switch -- {} = {:?}", name, switch);

        match switch {
            None => self.named.insert(name.into(), NamedValue::AbsentSwitch),
            Some(flag) => self
                .named
                .insert(name, NamedValue::PresentSwitch(*flag.name())),
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
