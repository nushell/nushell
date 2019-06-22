use crate::evaluate::{evaluate_baseline_expr, Scope};
use crate::parser::{hir, hir::ExpressionKindHint, parse_command, CallNode, Spanned};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use indexmap::IndexMap;
use log::trace;
use std::fmt;

#[allow(unused)]
#[derive(Debug)]
pub enum NamedType {
    Switch,
    Mandatory(NamedValue),
    Optional(NamedValue),
}

#[derive(Debug)]
pub enum NamedValue {
    Single,

    #[allow(unused)]
    Block,
}

impl NamedValue {
    crate fn to_coerce_hint(&self) -> Option<ExpressionKindHint> {
        match self {
            NamedValue::Single => None,
            NamedValue::Block => Some(ExpressionKindHint::Block),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum PositionalType {
    Value(String),
    Block(String),
}

impl PositionalType {
    crate fn to_coerce_hint(&self) -> Option<ExpressionKindHint> {
        match self {
            PositionalType::Value(_) => None,
            PositionalType::Block(_) => Some(ExpressionKindHint::Block),
        }
    }
}

#[derive(Debug, Getters)]
#[get = "crate"]
pub struct CommandConfig {
    crate name: String,
    crate mandatory_positional: Vec<PositionalType>,
    crate optional_positional: Vec<PositionalType>,
    crate rest_positional: bool,
    crate named: IndexMap<String, NamedType>,
}

#[derive(Debug, Default, new)]
pub struct Args {
    pub positional: Option<Vec<Spanned<Value>>>,
    pub named: Option<IndexMap<String, Spanned<Value>>>,
}

#[derive(new)]
pub struct DebugPositional<'a> {
    positional: &'a Option<Vec<Spanned<Value>>>,
}

impl fmt::Debug for DebugPositional<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.positional {
            None => write!(f, "None"),
            Some(positional) => f
                .debug_list()
                .entries(positional.iter().map(|p| p.item().debug()))
                .finish(),
        }
    }
}

#[derive(new)]
pub struct DebugNamed<'a> {
    named: &'a Option<IndexMap<String, Spanned<Value>>>,
}

impl fmt::Debug for DebugNamed<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.named {
            None => write!(f, "None"),
            Some(named) => f
                .debug_map()
                .entries(named.iter().map(|(k, v)| (k, v.item().debug())))
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

    pub fn nth(&self, pos: usize) -> Option<&Spanned<Value>> {
        match &self.positional {
            None => None,
            Some(array) => array.iter().nth(pos),
        }
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Spanned<Value>, ShellError> {
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

    pub fn get(&self, name: &str) -> Option<&Spanned<Value>> {
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
    Array(std::slice::Iter<'a, Spanned<Value>>),
}

impl Iterator for PositionalIter<'a> {
    type Item = &'a Spanned<Value>;

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
        call: &Spanned<CallNode>,
        registry: &dyn CommandRegistry,
        scope: &Scope,
        source: &Text,
    ) -> Result<Args, ShellError> {
        let args = parse_command(self, registry, call, source)?;

        trace!("parsed args: {:?}", args);

        evaluate_args(args, registry, scope, source)

        // let mut positional: Vec<Spanned<Value>> = vec![];
        // let mut named: IndexMap<String, Value> = IndexMap::default();

        // let mut args: Vec<TokenNode> = args.cloned().collect();

        // for (key, ty) in self.named.iter() {
        //     let index = args.iter().position(|a| a.is_flag(&key, source));

        //     match (index, ty) {
        //         (Some(i), NamedType::Switch) => {
        //             args.remove(i);
        //             named.insert(key.clone(), Value::boolean(true));
        //         }

        //         (None, NamedType::Switch) => {}

        //         (Some(i), NamedType::Optional(v)) => {
        //             args.remove(i);
        //             named.insert(key.clone(), extract_named(&mut args, i, v)?);
        //         }

        //         (None, NamedType::Optional(_)) => {}

        //         (Some(i), NamedType::Mandatory(v)) => {
        //             args.remove(i);
        //             named.insert(key.clone(), extract_named(&mut args, i, v)?);
        //         }

        //         (None, NamedType::Mandatory(_)) => {
        //             return Err(ShellError::string(&format!(
        //                 "Expected mandatory argument {}, but it was missing",
        //                 key
        //             )))
        //         }
        //     }
        // }

        // let mut args = args.into_iter();

        // for param in &self.mandatory_positional {
        //     let arg = args.next();

        //     let value = match arg {
        //         None => {
        //             return Err(ShellError::string(format!(
        //                 "expected mandatory positional argument {}",
        //                 param.name()
        //             )))
        //         }

        //         Some(arg) => param.evaluate(arg.clone(), scope, source)?,
        //     };

        //     positional.push(value);
        // }

        // if self.rest_positional {
        //     let rest: Result<Vec<Spanned<Value>>, _> = args
        //         .map(|i| evaluate_baseline_expr(&i, &Scope::empty(), source))
        //         .collect();
        //     positional.extend(rest?);
        // } else {
        //     let rest: Vec<TokenNode> = args.collect();

        //     if rest.len() > 0 {
        //         return Err(ShellError::string(&format!(
        //             "Too many arguments, extras: {:?}",
        //             rest
        //         )));
        //     }
        // }

        // Ok(Args { positional, named })
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

    let named: Result<Option<IndexMap<String, Spanned<Value>>>, ShellError> = args
        .named()
        .as_ref()
        .map(|n| {
            let mut results = IndexMap::new();

            for (name, value) in n.named.iter() {
                match value {
                    hir::named::NamedValue::PresentSwitch(span) => {
                        results.insert(
                            name.clone(),
                            Spanned::from_item(Value::boolean(true), *span),
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
