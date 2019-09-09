use crate::parser::TokenNode;
use crate::traits::ToDebug;
use getset::Getters;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters)]
pub struct CallNode {
    #[get = "pub(crate)"]
    head: Box<TokenNode>,
    #[get = "pub(crate)"]
    children: Option<Vec<TokenNode>>,
}

impl CallNode {
    pub fn new(head: Box<TokenNode>, children: Vec<TokenNode>) -> CallNode {
        if children.len() == 0 {
            CallNode {
                head,
                children: None,
            }
        } else {
            CallNode {
                head,
                children: Some(children),
            }
        }
    }
}

impl ToDebug for CallNode {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.head.debug(source))?;

        if let Some(children) = &self.children {
            for child in children {
                write!(f, "{}", child.debug(source))?
            }
        }

        Ok(())
    }
}
