use crate::value::{Primitive, UntaggedValue, Value};
use indexmap::IndexMap;
use nu_errors::ShellError;
use query_interface::{interfaces, vtable_for, Object, ObjectHash};
use serde::{Deserialize, Serialize};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt::Debug;

/// An evaluation scope. Scopes map variable names to Values and aid in evaluating blocks and expressions.
/// Additionally, holds the value for the special $it variable, a variable used to refer to the value passing
/// through the pipeline at that moment
#[derive(Debug)]
pub struct Scope {
    pub it: Value,
    pub vars: IndexMap<String, Value>,
}

impl Scope {
    /// Create a new scope
    pub fn new(it: Value) -> Scope {
        Scope {
            it,
            vars: IndexMap::new(),
        }
    }
}

impl Scope {
    /// Create an empty scope
    pub fn empty() -> Scope {
        Scope {
            it: UntaggedValue::Primitive(Primitive::Nothing).into_untagged_value(),
            vars: IndexMap::new(),
        }
    }

    /// Create an empty scope, setting $it to a known Value
    pub fn it_value(value: Value) -> Scope {
        Scope {
            it: value,
            vars: IndexMap::new(),
        }
    }
}

#[typetag::serde(tag = "type")]
pub trait EvaluateTrait: Debug + Send + Sync + Object + ObjectHash + 'static {
    fn invoke(&self, scope: &Scope) -> Result<Value, ShellError>;
    fn clone_box(&self) -> Evaluate;
}

interfaces!(Evaluate: dyn ObjectHash);

#[typetag::serde]
impl EvaluateTrait for Evaluate {
    fn invoke(&self, scope: &Scope) -> Result<Value, ShellError> {
        self.expr.invoke(scope)
    }

    fn clone_box(&self) -> Evaluate {
        self.expr.clone_box()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Evaluate {
    expr: Box<dyn EvaluateTrait>,
}

impl Evaluate {
    pub fn new(evaluate: impl EvaluateTrait) -> Evaluate {
        Evaluate {
            expr: Box::new(evaluate),
        }
    }
}

impl std::hash::Hash for Evaluate {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.expr.obj_hash(state)
    }
}

impl Clone for Evaluate {
    fn clone(&self) -> Evaluate {
        self.expr.clone_box()
    }
}

impl Ord for Evaluate {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl PartialOrd for Evaluate {
    fn partial_cmp(&self, _: &Evaluate) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl PartialEq for Evaluate {
    fn eq(&self, _: &Evaluate) -> bool {
        true
    }
}

impl Eq for Evaluate {}
