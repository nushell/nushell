// TODO: Temporary redirect
crate use crate::context::CommandRegistry;
use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, hir::SyntaxType, parse_command, CallNode};
use crate::prelude::*;
use derive_new::new;
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

#[derive(Debug, Serialize, Deserialize, Clone, new)]
pub struct Signature {
    pub name: String,
    #[new(default)]
    pub positional: Vec<PositionalType>,
    #[new(value = "false")]
    pub rest_positional: bool,
    #[new(default)]
    pub named: IndexMap<String, NamedType>,
    #[new(value = "false")]
    pub is_filter: bool,
}

impl Signature {
    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into())
    }

    pub fn required(mut self, name: impl Into<String>, ty: impl Into<SyntaxType>) -> Signature {
        self.positional
            .push(PositionalType::Mandatory(name.into(), ty.into()));

        self
    }

    pub fn optional(mut self, name: impl Into<String>, ty: impl Into<SyntaxType>) -> Signature {
        self.positional
            .push(PositionalType::Optional(name.into(), ty.into()));

        self
    }

    pub fn named(mut self, name: impl Into<String>, ty: impl Into<SyntaxType>) -> Signature {
        self.named
            .insert(name.into(), NamedType::Optional(ty.into()));

        self
    }

    pub fn required_named(
        mut self,
        name: impl Into<String>,
        ty: impl Into<SyntaxType>,
    ) -> Signature {
        self.named
            .insert(name.into(), NamedType::Mandatory(ty.into()));

        self
    }

    pub fn switch(mut self, name: impl Into<String>) -> Signature {
        self.named.insert(name.into(), NamedType::Switch);

        self
    }

    pub fn filter(mut self) -> Signature {
        self.is_filter = true;
        self
    }

    pub fn rest(mut self) -> Signature {
        self.rest_positional = true;
        self
    }
}

#[derive(Debug, Default, new, Serialize, Deserialize, Clone)]
pub struct EvaluatedArgs {
    pub positional: Option<Vec<Tagged<Value>>>,
    pub named: Option<IndexMap<String, Tagged<Value>>>,
}

#[derive(new)]
pub struct DebugEvaluatedPositional<'a> {
    positional: &'a Option<Vec<Tagged<Value>>>,
}

impl fmt::Debug for DebugEvaluatedPositional<'a> {
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
pub struct DebugEvaluatedNamed<'a> {
    named: &'a Option<IndexMap<String, Tagged<Value>>>,
}

impl fmt::Debug for DebugEvaluatedNamed<'a> {
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

pub struct DebugEvaluatedArgs<'a> {
    args: &'a EvaluatedArgs,
}

impl fmt::Debug for DebugEvaluatedArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = f.debug_struct("Args");

        s.field(
            "positional",
            &DebugEvaluatedPositional::new(&self.args.positional),
        );
        s.field("named", &DebugEvaluatedNamed::new(&self.args.named));

        s.finish()
    }
}

impl EvaluatedArgs {
    pub fn debug(&'a self) -> DebugEvaluatedArgs<'a> {
        DebugEvaluatedArgs { args: self }
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

impl Signature {
    crate fn parse_args(
        &self,
        call: &Tagged<CallNode>,
        registry: &CommandRegistry,
        source: &Text,
    ) -> Result<hir::Call, ShellError> {
        let args = parse_command(self, registry, call, source)?;

        trace!("parsed args: {:?}", args);

        Ok(args)
    }

    #[allow(unused)]
    crate fn signature(&self) -> String {
        format!("TODO")
    }
}

crate fn evaluate_args(
    call: &hir::Call,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<EvaluatedArgs, ShellError> {
    println!("positional (before): {:?}", call);
    let positional: Result<Option<Vec<_>>, _> = call
        .positional()
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|e| evaluate_baseline_expr(e, registry, scope, source))
                .collect()
        })
        .transpose();

    println!("positional: {:?}", positional);
    let positional = positional?;

    let named: Result<Option<IndexMap<String, Tagged<Value>>>, ShellError> = call
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

    Ok(EvaluatedArgs::new(positional, named))
}
