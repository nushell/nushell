use crate::{ModuleId, Span};
use std::collections::HashMap;

/// Organizes documentation comments for various primitives
#[derive(Debug, Clone)]
pub(super) struct Doccomments {
    // TODO: Move decl doccomments here
    module_comments: HashMap<ModuleId, Vec<Span>>,
}

impl Doccomments {
    pub fn new() -> Self {
        Doccomments {
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
    pub fn merge_with(&mut self, other: Doccomments) {
        self.module_comments.extend(other.module_comments);
    }
}

impl Default for Doccomments {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn build_desc(comment_lines: &[&[u8]]) -> (String, String) {
    let mut description = String::new();

    let mut num_spaces = 0;
    let mut first = true;

    // Use the comments to build the item/command description
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

        if !description.is_empty() {
            description.push('\n');
        }
        description.push_str(&comment_line);
    }

    if let Some((brief_desc, extra_desc)) = description.split_once("\r\n\r\n") {
        (brief_desc.to_string(), extra_desc.to_string())
    } else if let Some((brief_desc, extra_desc)) = description.split_once("\n\n") {
        (brief_desc.to_string(), extra_desc.to_string())
    } else {
        (description, String::default())
    }
}
