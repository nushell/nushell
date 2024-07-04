use crate::{ModuleId, Span};
use std::collections::HashMap;

/// Organizes usage messages for various primitives
#[derive(Debug, Clone)]
pub(super) struct Usage {
    // TODO: Move decl usages here
    module_comments: HashMap<ModuleId, Vec<Span>>,
}

impl Usage {
    pub fn new() -> Self {
        Usage {
            module_comments: HashMap::new(),
        }
    }

    pub fn add_module_comments(&mut self, module_id: ModuleId, comments: Vec<Span>) {
        self.module_comments.insert(module_id, comments);
    }

    pub fn get_module_comments(&self, module_id: ModuleId) -> Option<&[Span]> {
        self.module_comments.get(&module_id).map(|v| v.as_ref())
    }

    /// Overwrite own values with the other
    pub fn merge_with(&mut self, other: Usage) {
        self.module_comments.extend(other.module_comments);
    }
}

impl Default for Usage {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn build_usage(comment_lines: &[&[u8]]) -> (String, String) {
    let mut usage = String::new();

    let mut num_spaces = 0;
    let mut first = true;

    // Use the comments to build the usage
    for contents in comment_lines {
        let comment_line = if first {
            // Count the number of spaces still at the front, skipping the '#'
            let mut pos = 1;
            while pos < contents.len() {
                if let Some(b' ') = contents.get(pos) {
                    // continue
                } else {
                    break;
                }
                pos += 1;
            }

            num_spaces = pos;

            first = false;

            String::from_utf8_lossy(&contents[pos..]).to_string()
        } else {
            let mut pos = 1;

            while pos < contents.len() && pos < num_spaces {
                if let Some(b' ') = contents.get(pos) {
                    // continue
                } else {
                    break;
                }
                pos += 1;
            }

            String::from_utf8_lossy(&contents[pos..]).to_string()
        };

        if !usage.is_empty() {
            usage.push('\n');
        }
        usage.push_str(&comment_line);
    }

    if let Some((brief_usage, extra_usage)) = usage.split_once("\r\n\r\n") {
        (brief_usage.to_string(), extra_usage.to_string())
    } else if let Some((brief_usage, extra_usage)) = usage.split_once("\n\n") {
        (brief_usage.to_string(), extra_usage.to_string())
    } else {
        (usage, String::default())
    }
}
