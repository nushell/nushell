use crate::parser::hir::Expression;
use crate::parser::Flag;
use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;
use log::trace;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Tag),
    AbsentValue,
    Value(Expression),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, new)]
pub struct NamedArguments {
    #[new(default)]
    pub(crate) named: IndexMap<String, NamedValue>,
}

impl ToDebug for NamedArguments {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        for (name, value) in &self.named {
            match value {
                NamedValue::AbsentSwitch => continue,
                NamedValue::PresentSwitch(tag) => write!(f, " --{}", tag.slice(source))?,
                NamedValue::AbsentValue => continue,
                NamedValue::Value(expr) => write!(f, " --{} {}", name, expr.debug(source))?,
            }
        }

        Ok(())
    }
}

impl NamedArguments {
    pub fn insert_switch(&mut self, name: impl Into<String>, switch: Option<Flag>) {
        let name = name.into();
        trace!("Inserting switch -- {} = {:?}", name, switch);

        match switch {
            None => self.named.insert(name.into(), NamedValue::AbsentSwitch),
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
