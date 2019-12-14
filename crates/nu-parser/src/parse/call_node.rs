use crate::parse::token_tree::SpannedToken;
use getset::Getters;
use nu_source::{b, DebugDocBuilder, PrettyDebugWithSource};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Getters)]
pub struct CallNode {
    #[get = "pub(crate)"]
    head: Box<SpannedToken>,
    #[get = "pub(crate)"]
    children: Option<Vec<SpannedToken>>,
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
    pub fn new(head: Box<SpannedToken>, children: Vec<SpannedToken>) -> CallNode {
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
