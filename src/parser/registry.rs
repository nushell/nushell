use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, hir::SyntaxType, parse_command, CallNode};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use log::trace;
use serde::{Deserialize, Serialize};
use std::fmt;

#[allow(unused)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NamedType {
    Switch,
    Mandatory(SyntaxType),
    Optional(SyntaxType),
}

#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionalType {
    Mandatory(String, SyntaxType),
    Optional(String, SyntaxType),
}

impl PositionalType {
    pub fn mandatory(name: &str, ty: SyntaxType) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), ty)
    }

    pub fn mandatory_any(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxType::Any)
    }

    pub fn mandatory_block(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxType::Block)
    }

    pub fn optional(name: &str, ty: SyntaxType) -> PositionalType {
        PositionalType::Optional(name.to_string(), ty)
    }

    pub fn optional_any(name: &str) -> PositionalType {
        PositionalType::Optional(name.to_string(), SyntaxType::Any)
    }

    #[allow(unused)]
    crate fn to_coerce_hint(&self) -> Option<SyntaxType> {
        match self {
            PositionalType::Mandatory(_, SyntaxType::Block)
            | PositionalType::Optional(_, SyntaxType::Block) => Some(SyntaxType::Block),
            _ => None,
        }
    }

    crate fn name(&self) -> &str {
        match self {
            PositionalType::Mandatory(s, _) => s,
            PositionalType::Optional(s, _) => s,
        }
    }

    crate fn syntax_type(&self) -> SyntaxType {
        match *self {
            PositionalType::Mandatory(_, t) => t,
            PositionalType::Optional(_, t) => t,
        }
    }
}

#[derive(Debug, Getters, Serialize, Deserialize, Clone)]
#[get = "crate"]
pub struct CommandConfig {
    pub name: String,
    pub positional: Vec<PositionalType>,
    pub rest_positional: bool,
    pub named: IndexMap<String, NamedType>,
    pub is_filter: bool,
    pub is_sink: bool,
}

#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct Args {
    pub positional: Option<Vec<Tagged<Value>>>,
    pub named: Option<IndexMap<String, Tagged<Value>>>,
}

#[derive(new)]
pub struct DebugPositional<'a> {
    positional: &'a Option<Vec<Tagged<Value>>>,
}

impl fmt::Debug for DebugPositional<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.positional {
            None => write!(f, "None"),
            Some(positional) => f
                .debug_list()
                .entries(positional.iter().map(|p| p.debug()))
                .finish(),
        }
    }
}

#[derive(new)]
pub struct DebugNamed<'a> {
    named: &'a Option<IndexMap<String, Tagged<Value>>>,
}

impl fmt::Debug for DebugNamed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.named {
            None => write!(f, "None"),
            Some(named) => f
                .debug_map()
                .entries(named.iter().map(|(k, v)| (k, v.debug())))
                .finish(),
        }
    }
}

pub struct DebugArgs<'a> {
    args: &'a Args,
}

impl fmt::Debug for DebugArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = f.debug_struct("Args");

        s.field("positional", &DebugPositional::new(&self.args.positional));
        s.field("named", &DebugNamed::new(&self.args.named));

        s.finish()
    }
}

impl Args {
    pub fn debug(&'a self) -> DebugArgs<'a> {
        DebugArgs { args: self }
    }

    pub fn nth(&self, pos: usize) -> Option<&Tagged<Value>> {
        match &self.positional {
            None => None,
            Some(array) => array.iter().nth(pos),
        }
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Tagged<Value>, ShellError> {
        match &self.positional {
            None => Err(ShellError::unimplemented("Better error: expect_nth")),
            Some(array) => match array.iter().nth(pos) {
                None => Err(ShellError::unimplemented("Better error: expect_nth")),
                Some(item) => Ok(item),
            },
        }
    }

    pub fn len(&self) -> usize {
        match &self.positional {
            None => 0,
            Some(array) => array.len(),
        }
    }

    pub fn has(&self, name: &str) -> bool {
        match &self.named {
            None => false,
            Some(named) => named.contains_key(name),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Tagged<Value>> {
        match &self.named {
            None => None,
            Some(named) => named.get(name),
        }
    }

    pub fn positional_iter(&'a self) -> PositionalIter<'a> {
        match &self.positional {
            None => PositionalIter::Empty,
            Some(v) => {
                let iter = v.iter();
                PositionalIter::Array(iter)
            }
        }
    }
}

pub enum PositionalIter<'a> {
    Empty,
    Array(std::slice::Iter<'a, Tagged<Value>>),
}

impl Iterator for PositionalIter<'a> {
    type Item = &'a Tagged<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PositionalIter::Empty => None,
            PositionalIter::Array(iter) => iter.next(),
        }
    }
}

impl CommandConfig {
    crate fn evaluate_args(
        &self,
        call: &Tagged<CallNode>,
        registry: &dyn CommandRegistry,
        scope: &Scope,
        source: &Text,
    ) -> Result<Args, ShellError> {
        let args = parse_command(self, registry, call, source)?;

        trace!("parsed args: {:?}", args);

        evaluate_args(args, registry, scope, source)
    }

    #[allow(unused)]
    crate fn signature(&self) -> String {
        format!("TODO")
    }
}

fn evaluate_args(
    args: hir::Call,
    registry: &dyn CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<Args, ShellError> {
    let positional: Result<Option<Vec<_>>, _> = args
        .positional()
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|e| evaluate_baseline_expr(e, &(), scope, source))
                .collect()
        })
        .transpose();

    let positional = positional?;

    let named: Result<Option<IndexMap<String, Tagged<Value>>>, ShellError> = args
        .named()
        .as_ref()
        .map(|n| {
            let mut results = IndexMap::new();

            for (name, value) in n.named.iter() {
                match value {
                    hir::named::NamedValue::PresentSwitch(span) => {
                        results.insert(
                            name.clone(),
                            Tagged::from_simple_spanned_item(Value::boolean(true), *span),
                        );
                    }
                    hir::named::NamedValue::Value(expr) => {
                        results.insert(
                            name.clone(),
                            evaluate_baseline_expr(expr, registry, scope, source)?,
                        );
                    }

                    _ => {}
                };
            }

            Ok(results)
        })
        .transpose();

    let named = named?;

    Ok(Args::new(positional, named))
}

pub trait CommandRegistry {
    fn get(&self, name: &str) -> Option<CommandConfig>;
}

impl CommandRegistry for () {
    fn get(&self, _name: &str) -> Option<CommandConfig> {
        None
    }
}
