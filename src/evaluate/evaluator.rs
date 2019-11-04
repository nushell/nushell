use crate::data::base::Block;
use crate::errors::ArgumentError;
use crate::parser::hir::path::{ColumnPath, RawPathMember};
use crate::parser::{
    hir::{self, Expression, RawExpression},
    CommandRegistry, Text,
};
use crate::prelude::*;
use crate::TaggedDictBuilder;
use indexmap::IndexMap;
use log::trace;
use std::fmt;

pub struct Scope {
    it: Tagged<Value>,
    vars: IndexMap<String, Tagged<Value>>,
}

impl Scope {
    pub fn new(it: Tagged<Value>) -> Scope {
        Scope {
            it,
            vars: IndexMap::new(),
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entry(&"$it", &format!("{:?}", self.it.item))
            .entries(self.vars.iter().map(|(k, v)| (k, &v.item)))
            .finish()
    }
}

impl Scope {
    pub(crate) fn empty() -> Scope {
        Scope {
            it: Value::nothing().tagged_unknown(),
            vars: IndexMap::new(),
        }
    }

    pub(crate) fn it_value(value: Tagged<Value>) -> Scope {
        Scope {
            it: value,
            vars: IndexMap::new(),
        }
    }
}

pub(crate) fn evaluate_baseline_expr(
    expr: &Expression,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<Tagged<Value>, ShellError> {
    let tag = Tag {
        span: expr.span,
        anchor: None,
    };
    match &expr.item {
        RawExpression::Literal(literal) => Ok(evaluate_literal(literal.tagged(tag), source)),
        RawExpression::ExternalWord => Err(ShellError::argument_error(
            "Invalid external word".spanned(tag.span),
            ArgumentError::InvalidExternalWord,
        )),
        RawExpression::FilePath(path) => Ok(Value::path(path.clone()).tagged(tag)),
        RawExpression::Synthetic(hir::Synthetic::String(s)) => {
            Ok(Value::string(s).tagged_unknown())
        }
        RawExpression::Variable(var) => evaluate_reference(var, scope, source, tag),
        RawExpression::Command(_) => evaluate_command(tag, scope, source),
        RawExpression::ExternalCommand(external) => evaluate_external(external, scope, source),
        RawExpression::Binary(binary) => {
            let left = evaluate_baseline_expr(binary.left(), registry, scope, source)?;
            let right = evaluate_baseline_expr(binary.right(), registry, scope, source)?;

            trace!("left={:?} right={:?}", left.item, right.item);

            match left.compare(binary.op(), &*right) {
                Ok(result) => Ok(Value::boolean(result).tagged(tag)),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    left_type.spanned(binary.left().span),
                    right_type.spanned(binary.right().span),
                )),
            }
        }
        RawExpression::List(list) => {
            let mut exprs = vec![];

            for expr in list {
                let expr = evaluate_baseline_expr(expr, registry, scope, source)?;
                exprs.push(expr);
            }

            Ok(Value::Table(exprs).tagged(tag))
        }
        RawExpression::Block(block) => {
            Ok(Value::Block(Block::new(block.clone(), source.clone(), tag.clone())).tagged(&tag))
        }
        RawExpression::Path(path) => {
            let value = evaluate_baseline_expr(path.head(), registry, scope, source)?;
            let mut item = value;

            for member in path.tail() {
                let next = item.get_data_by_member(member);

                match next {
                    Err(err) => {
                        let possibilities = item.data_descriptors();

                        if let RawPathMember::String(name) = &member.item {
                            let mut possible_matches: Vec<_> = possibilities
                                .iter()
                                .map(|x| (natural::distance::levenshtein_distance(x, &name), x))
                                .collect();

                            possible_matches.sort();

                            if possible_matches.len() > 0 {
                                return Err(ShellError::labeled_error(
                                    "Unknown column",
                                    format!("did you mean '{}'?", possible_matches[0].1),
                                    &tag,
                                ));
                            } else {
                                return Err(err);
                            }
                        }
                    }
                    Ok(next) => {
                        item = next.clone().item.tagged(&tag);
                    }
                };
            }

            Ok(item.item().clone().tagged(tag))
        }
        RawExpression::Boolean(_boolean) => unimplemented!(),
    }
}

fn evaluate_literal(literal: Tagged<&hir::Literal>, source: &Text) -> Tagged<Value> {
    let result = match literal.item {
        hir::Literal::ColumnPath(path) => {
            let members = path
                .iter()
                .map(|member| member.to_path_member(source))
                .collect();

            Value::Primitive(Primitive::ColumnPath(ColumnPath::new(members)))
        }
        hir::Literal::Number(int) => int.into(),
        hir::Literal::Size(int, unit) => unit.compute(int),
        hir::Literal::String(tag) => Value::string(tag.slice(source)),
        hir::Literal::GlobPattern(pattern) => Value::pattern(pattern),
        hir::Literal::Bare => Value::string(literal.tag().slice(source)),
    };

    literal.map(|_| result)
}

fn evaluate_reference(
    name: &hir::Variable,
    scope: &Scope,
    source: &Text,
    tag: Tag,
) -> Result<Tagged<Value>, ShellError> {
    trace!("Evaluating {} with Scope {}", name, scope);
    match name {
        hir::Variable::It(_) => Ok(scope.it.item.clone().tagged(tag)),
        hir::Variable::Other(inner) => match inner.slice(source) {
            x if x == "nu:env" => {
                let mut dict = TaggedDictBuilder::new(&tag);
                for v in std::env::vars() {
                    if v.0 != "PATH" && v.0 != "Path" {
                        dict.insert(v.0, Value::string(v.1));
                    }
                }
                Ok(dict.into_tagged_value())
            }
            x if x == "nu:config" => {
                let config = crate::data::config::read(tag.clone(), &None)?;
                Ok(Value::row(config).tagged(tag))
            }
            x if x == "nu:path" => {
                let mut table = vec![];
                match std::env::var_os("PATH") {
                    Some(paths) => {
                        for path in std::env::split_paths(&paths) {
                            table.push(Value::path(path).tagged(&tag));
                        }
                    }
                    _ => {}
                }
                Ok(Value::table(&table).tagged(tag))
            }
            x => Ok(scope
                .vars
                .get(x)
                .map(|v| v.clone())
                .unwrap_or_else(|| Value::nothing().tagged(tag))),
        },
    }
}

fn evaluate_external(
    external: &hir::ExternalCommand,
    _scope: &Scope,
    _source: &Text,
) -> Result<Tagged<Value>, ShellError> {
    Err(ShellError::syntax_error(
        "Unexpected external command".spanned(*external.name()),
    ))
}

fn evaluate_command(tag: Tag, _scope: &Scope, _source: &Text) -> Result<Tagged<Value>, ShellError> {
    Err(ShellError::syntax_error(
        "Unexpected command".spanned(tag.span),
    ))
}
