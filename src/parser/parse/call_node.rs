use crate::parser::TokenNode;
use getset::Getters;

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
