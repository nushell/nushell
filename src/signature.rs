use crate::{parser::SyntaxShape, Declaration, VarId};

#[derive(Debug, Clone)]
pub struct Flag {
    pub long: String,
    pub short: Option<char>,
    pub arg: Option<SyntaxShape>,
    pub required: bool,
    pub desc: String,
    // For custom commands
    pub var_id: Option<VarId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionalArg {
    pub name: String,
    pub desc: String,
    pub shape: SyntaxShape,
    // For custom commands
    pub var_id: Option<VarId>,
}

#[derive(Clone, Debug)]
pub struct Signature {
    pub name: String,
    pub usage: String,
    pub extra_usage: String,
    pub required_positional: Vec<PositionalArg>,
    pub optional_positional: Vec<PositionalArg>,
    pub rest_positional: Option<PositionalArg>,
    pub named: Vec<Flag>,
    pub is_filter: bool,
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.usage == other.usage
            && self.required_positional == other.required_positional
            && self.optional_positional == other.optional_positional
            && self.rest_positional == other.rest_positional
            && self.is_filter == other.is_filter
    }
}

impl Eq for Signature {}

impl Signature {
    pub fn new(name: impl Into<String>) -> Signature {
        Signature {
            name: name.into(),
            usage: String::new(),
            extra_usage: String::new(),
            required_positional: vec![],
            optional_positional: vec![],
            rest_positional: None,
            named: vec![],
            is_filter: false,
        }
    }
    pub fn build(name: impl Into<String>) -> Signature {
        Signature::new(name.into())
    }

    /// Add a description to the signature
    pub fn desc(mut self, usage: impl Into<String>) -> Signature {
        self.usage = usage.into();
        self
    }

    /// Add a required positional argument to the signature
    pub fn required(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.required_positional.push(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
        });

        self
    }

    /// Add a required positional argument to the signature
    pub fn optional(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
    ) -> Signature {
        self.optional_positional.push(PositionalArg {
            name: name.into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
        });

        self
    }

    pub fn rest(mut self, shape: impl Into<SyntaxShape>, desc: impl Into<String>) -> Signature {
        self.rest_positional = Some(PositionalArg {
            name: "rest".into(),
            desc: desc.into(),
            shape: shape.into(),
            var_id: None,
        });

        self
    }

    /// Add an optional named flag argument to the signature
    pub fn named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let s = short.map(|c| {
            debug_assert!(!self.get_shorts().contains(&c));
            c
        });
        self.named.push(Flag {
            long: name.into(),
            short: s,
            arg: Some(shape.into()),
            required: false,
            desc: desc.into(),
            var_id: None,
        });

        self
    }

    /// Add a required named flag argument to the signature
    pub fn required_named(
        mut self,
        name: impl Into<String>,
        shape: impl Into<SyntaxShape>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let s = short.map(|c| {
            debug_assert!(!self.get_shorts().contains(&c));
            c
        });
        self.named.push(Flag {
            long: name.into(),
            short: s,
            arg: Some(shape.into()),
            required: true,
            desc: desc.into(),
            var_id: None,
        });

        self
    }

    /// Add a switch to the signature
    pub fn switch(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        short: Option<char>,
    ) -> Signature {
        let s = short.map(|c| {
            debug_assert!(
                !self.get_shorts().contains(&c),
                "There may be duplicate short flags, such as -h"
            );
            c
        });

        self.named.push(Flag {
            long: name.into(),
            short: s,
            arg: None,
            required: false,
            desc: desc.into(),
            var_id: None,
        });
        self
    }

    /// Get list of the short-hand flags
    pub fn get_shorts(&self) -> Vec<char> {
        let mut shorts = Vec::new();
        for Flag { short, .. } in &self.named {
            if let Some(c) = short {
                shorts.push(*c);
            }
        }
        shorts
    }

    pub fn get_positional(&self, position: usize) -> Option<PositionalArg> {
        if position < self.required_positional.len() {
            self.required_positional.get(position).cloned()
        } else if position < (self.required_positional.len() + self.optional_positional.len()) {
            self.optional_positional
                .get(position - self.required_positional.len())
                .cloned()
        } else {
            self.rest_positional.clone()
        }
    }

    pub fn num_positionals(&self) -> usize {
        let mut total = self.required_positional.len() + self.optional_positional.len();

        for positional in &self.required_positional {
            if let SyntaxShape::Keyword(..) = positional.shape {
                // Keywords have a required argument, so account for that
                total += 1;
            }
        }
        for positional in &self.optional_positional {
            if let SyntaxShape::Keyword(..) = positional.shape {
                // Keywords have a required argument, so account for that
                total += 1;
            }
        }
        total
    }

    pub fn num_positionals_after(&self, idx: usize) -> usize {
        let mut total = 0;
        let mut curr = 0;

        for positional in &self.required_positional {
            match positional.shape {
                SyntaxShape::Keyword(..) => {
                    // Keywords have a required argument, so account for that
                    if curr > idx {
                        total += 2;
                    }
                }
                _ => {
                    if curr > idx {
                        total += 1;
                    }
                }
            }
            curr += 1;
        }
        for positional in &self.optional_positional {
            match positional.shape {
                SyntaxShape::Keyword(..) => {
                    // Keywords have a required argument, so account for that
                    if curr > idx {
                        total += 2;
                    }
                }
                _ => {
                    if curr > idx {
                        total += 1;
                    }
                }
            }
            curr += 1;
        }
        total
    }

    /// Find the matching long flag
    pub fn get_long_flag(&self, name: &str) -> Option<Flag> {
        for flag in &self.named {
            if flag.long == name {
                return Some(flag.clone());
            }
        }
        None
    }

    /// Find the matching long flag
    pub fn get_short_flag(&self, short: char) -> Option<Flag> {
        for flag in &self.named {
            if let Some(short_flag) = &flag.short {
                if *short_flag == short {
                    return Some(flag.clone());
                }
            }
        }
        None
    }
}

impl From<Box<Signature>> for Declaration {
    fn from(val: Box<Signature>) -> Self {
        Declaration {
            signature: val,
            body: None,
        }
    }
}

impl From<Signature> for Declaration {
    fn from(val: Signature) -> Self {
        Declaration {
            signature: Box::new(val),
            body: None,
        }
    }
}
