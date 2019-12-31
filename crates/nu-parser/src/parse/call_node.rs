use crate::TokenNode;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters)]
pub struct CallNode {
    #[get = "pub(crate)"]
    head: Box<TokenNode>,
    #[get = "pub(crate)"]
    children: Option<Vec<TokenNode>>,
}

impl PrettyDebugWithSource for CallNode {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "call",
            self.head.pretty_debug(source)
                + b::preceded(
                    b::space(),
                    b::intersperse(
                        self.children.iter().flat_map(|children| {
                            children.iter().map(|child| child.pretty_debug(source))
                        }),
                        b::space(),
                    ),
                ),
        )
    }
}

impl CallNode {
    pub fn new(head: Box<TokenNode>, children: Vec<TokenNode>) -> CallNode {
        if children.is_empty() {
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
