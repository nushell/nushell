use crate::hir::SpannedExpression;
use crate::Flag;
use indexmap::IndexMap;
use log::trace;
use nu_source::{b, DebugDocBuilder, PrettyDebugRefineKind, PrettyDebugWithSource, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NamedValue {
    AbsentSwitch,
    PresentSwitch(Tag),
    AbsentValue,
    Value(SpannedExpression),
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

    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => match self {
                NamedValue::AbsentSwitch => b::value("absent"),
                NamedValue::PresentSwitch(_) => b::value("present"),
                NamedValue::AbsentValue => b::value("absent"),
                NamedValue::Value(value) => value.refined_pretty_debug(refine, source),
            },
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

    pub fn get(&self, name: &str) -> Option<&NamedValue> {
        self.named.get(name)
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

    pub fn insert_optional(&mut self, name: impl Into<String>, expr: Option<SpannedExpression>) {
        match expr {
            None => self.named.insert(name.into(), NamedValue::AbsentValue),
            Some(expr) => self.named.insert(name.into(), NamedValue::Value(expr)),
        };
    }

    pub fn insert_mandatory(&mut self, name: impl Into<String>, expr: SpannedExpression) {
        self.named.insert(name.into(), NamedValue::Value(expr));
    }

    pub fn switch_present(&self, switch: &str) -> bool {
        self.named
            .get(switch)
            .map(|t| match t {
                NamedValue::PresentSwitch(_) => true,
                _ => false,
            })
            .unwrap_or(false)
    }
}

impl PrettyDebugWithSource for NamedArguments {
    fn refined_pretty_debug(&self, refine: PrettyDebugRefineKind, source: &str) -> DebugDocBuilder {
        match refine {
            PrettyDebugRefineKind::ContextFree => self.pretty_debug(source),
            PrettyDebugRefineKind::WithContext => b::intersperse(
                self.named.iter().map(|(key, value)| {
                    b::key(key)
                        + b::equals()
                        + value.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source)
                }),
                b::space(),
            ),
        }
    }

    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::delimit(
            "(",
            self.refined_pretty_debug(PrettyDebugRefineKind::WithContext, source),
            ")",
        )
    }
}
