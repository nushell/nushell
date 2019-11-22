// TODO: Temporary redirect
pub(crate) use crate::context::CommandRegistry;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, hir::SyntaxShape};
use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NamedType {
    Switch,
    Mandatory(SyntaxShape),
    Optional(SyntaxShape),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionalType {
    Mandatory(String, SyntaxShape),
    Optional(String, SyntaxShape),
}

impl PositionalType {
    pub fn mandatory(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), ty)
    }

    pub fn mandatory_any(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Any)
    }

    pub fn mandatory_block(name: &str) -> PositionalType {
        PositionalType::Mandatory(name.to_string(), SyntaxShape::Block)
    }

    pub fn optional(name: &str, ty: SyntaxShape) -> PositionalType {
        PositionalType::Optional(name.to_string(), ty)
    }

    pub fn optional_any(name: &str) -> PositionalType {
        PositionalType::Optional(name.to_string(), SyntaxShape::Any)
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            PositionalType::Mandatory(s, _) => s,
            PositionalType::Optional(s, _) => s,
        }
    }

    pub(crate) fn syntax_type(&self) -> SyntaxShape {
        match *self {
            PositionalType::Mandatory(_, t) => t,
            PositionalType::Optional(_, t) => t,
        }
    }
}

type Description = String;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Signature {
    pub name: String,
    pub usage: String,
    pub positional: Vec<(PositionalType, Description)>,
    pub rest_positional: Option<(SyntaxShape, Description)>,
    pub named: IndexMap<String, (NamedType, Description)>,
    pub is_filter: bool,
}

impl Signature {
    pub fn new(name: String) -> Signature {
        Signature {
            name,
            usage: String::new(),
            positional: vec![],
            rest_positional: None,
            named: IndexMap::new(),
            is_filter: false,
        }
    }

    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into())
    }

    pub fn desc(mut self, usage: impl Into<String>) -> Signature {
        self.usage = usage.into();
        self
    }

    pub fn required(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Mandatory(name.into(), ty.into()),
            desc.into(),
        ));

        self
    }

    pub fn optional(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.positional.push((
            PositionalType::Optional(name.into(), ty.into()),
            desc.into(),
        ));

        self
    }

    pub fn named(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Optional(ty.into()), desc.into()));

        self
    }

    pub fn required_named(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Mandatory(ty.into()), desc.into()));

        self
    }

    pub fn switch(mut self, name: impl Into<String>, desc: impl Into<String>) -> Signature {
        self.named
            .insert(name.into(), (NamedType::Switch, desc.into()));

        self
    }

    pub fn filter(mut self) -> Signature {
        self.is_filter = true;
        self
    }

    pub fn rest(mut self, ty: SyntaxShape, desc: impl Into<String>) -> Signature {
        self.rest_positional = Some((ty, desc.into()));
        self
    }
}

#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct EvaluatedArgs {
    pub positional: Option<Vec<Tagged<Value>>>,
    pub named: Option<IndexMap<String, Tagged<Value>>>,
}

impl EvaluatedArgs {
    pub fn slice_from(&self, from: usize) -> Vec<Tagged<Value>> {
        let positional = &self.positional;

        match positional {
            None => vec![],
            Some(list) => list[from..].to_vec(),
        }
    }
}

impl EvaluatedArgs {
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

    pub fn positional_iter(&self) -> PositionalIter<'_> {
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

impl<'a> Iterator for PositionalIter<'a> {
    type Item = &'a Tagged<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PositionalIter::Empty => None,
            PositionalIter::Array(iter) => iter.next(),
        }
    }
}

pub(crate) fn evaluate_args(
    call: &hir::Call,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<EvaluatedArgs, ShellError> {
    let positional: Result<Option<Vec<_>>, _> = call
        .positional()
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|e| evaluate_baseline_expr(e, registry, scope, source))
                .collect()
        })
        .transpose();

    let positional = positional?;

    let named: Result<Option<IndexMap<String, Tagged<Value>>>, ShellError> = call
        .named()
        .as_ref()
        .map(|n| {
            let mut results = IndexMap::new();

            for (name, value) in n.named.iter() {
                match value {
                    hir::named::NamedValue::PresentSwitch(tag) => {
                        results.insert(name.clone(), Value::boolean(true).tagged(tag));
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

    Ok(EvaluatedArgs::new(positional, named))
}
