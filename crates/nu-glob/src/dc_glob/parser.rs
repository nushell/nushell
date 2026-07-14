use std::{
    ffi::OsStr,
    path::{Component, Path, PathBuf, is_separator},
};

#[derive(Debug, Clone)]
pub struct Pattern {
    pub nodes: Vec<AstNode>,
}

#[derive(Debug, Clone)]
pub enum AstNode {
    Separator,
    Prefix(String),
    RootDir,
    CurDir,
    ParentDir,
    Recurse,
    LiteralString(Vec<u8>),
    AnyCharacter,
    Wildcard,
    Characters(Vec<CharacterClass>),
    Alternatives {
        choices: Vec<Pattern>,
    },
    Repeat {
        min: u32,
        max: u32,
        pattern: Pattern,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterClass {
    Single(char),
    Range(char, char),
}

pub fn parse(string: impl AsRef<OsStr>) -> Pattern {
    let path = Path::new(string.as_ref());
    let mut components_iter = path.components().peekable();

    // Split the path into prefix components (where no glob pattern is allowed) and others
    let mut nodes = vec![];
    let mut path_relative = PathBuf::new();
    while let Some(Component::Prefix(..) | Component::RootDir) = components_iter.peek() {
        nodes.push(match components_iter.next() {
            Some(Component::Prefix(prefix_component)) => {
                AstNode::Prefix(prefix_component.as_os_str().to_string_lossy().into_owned())
            }
            Some(Component::RootDir) => AstNode::RootDir,
            _ => unreachable!(),
        });
    }
    path_relative.extend(components_iter);

    // Parse the remainder of the path into nodes
    parse_nodes(
        path_relative.as_os_str().as_encoded_bytes(),
        |_| true,
        &mut nodes,
    );

    Pattern { nodes }
}

pub fn parse_nodes<'a>(
    mut string: &'a [u8],
    mut cond: impl FnMut(&[u8]) -> bool,
    out: &mut Vec<AstNode>,
) -> &'a [u8] {
    while !string.is_empty() && cond(string) {
        string = next_node(string, out);
    }
    string
}

pub fn next_node<'a>(string: &'a [u8], out: &mut Vec<AstNode>) -> &'a [u8] {
    match node_separator((string, out))
        .or_else(node_any_character)
        .or_else(node_recurse)
        .or_else(node_wildcard)
        .or_else(node_alternatives)
        .or_else(node_character_class)
        .or_else(node_repeat)
        .or_else(node_cur_or_parent_dir)
        .or_else(node_literal_string)
    {
        Ok((remaining, _)) => remaining,
        Err((remaining, out)) => {
            if !remaining.is_empty() {
                out.push(AstNode::LiteralString(remaining.into()));
            }
            b""
        }
    }
}

type NodeInput<'a, 'b> = (&'a [u8], &'b mut Vec<AstNode>);
type NodeResult<'a, 'b> = std::result::Result<NodeInput<'a, 'b>, NodeInput<'a, 'b>>;

fn node_separator<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    match get_utf8_char(string) {
        Some((ch, next_string)) if is_separator(ch) => {
            out.push(AstNode::Separator);
            Ok((next_string, out))
        }
        _ => Err((string, out)),
    }
}

fn node_any_character<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    if string.first() == Some(&b'?') {
        out.push(AstNode::AnyCharacter);
        Ok((&string[1..], out))
    } else {
        Err((string, out))
    }
}

fn node_recurse<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    if string.get(0..2) == Some(b"**") {
        out.push(AstNode::Recurse);
        Ok((&string[2..], out))
    } else {
        Err((string, out))
    }
}

fn node_wildcard<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    if string.first() == Some(&b'*') {
        out.push(AstNode::Wildcard);
        Ok((&string[1..], out))
    } else {
        Err((string, out))
    }
}

fn node_alternatives<'a, 'b>((mut string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    let original_string = string;
    let mut choices = vec![];
    let mut current_out = vec![];
    if string.first() == Some(&b'{') {
        string = &string[1..];
        loop {
            string = parse_nodes(
                string,
                |string| !matches!(string.first(), Some(b',' | b'}')),
                &mut current_out,
            );
            match string.first() {
                Some(b',') => {
                    string = &string[1..];
                    let nodes = std::mem::take(&mut current_out);
                    choices.push(Pattern { nodes });
                }
                Some(b'}') => {
                    string = &string[1..];
                    choices.push(Pattern { nodes: current_out });
                    break;
                }
                Some(_) => continue,
                None => {
                    return Err((original_string, out));
                }
            }
        }
        out.push(AstNode::Alternatives { choices });
        Ok((string, out))
    } else {
        Err((original_string, out))
    }
}

fn node_character_class<'a, 'b>((mut string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    let original_string = string;
    if string.first() == Some(&b'[') {
        string = &string[1..];
        let mut classes = vec![];
        loop {
            let Some((start_char, next_string)) = get_utf8_char(string) else {
                return Err((original_string, out));
            };
            string = next_string;
            let ch_class = if string.first() == Some(&b'-') {
                // This is a range, due to the - char
                string = &string[1..];
                let Some((end_char, next_string)) = get_utf8_char(string) else {
                    return Err((original_string, out));
                };
                string = next_string;
                CharacterClass::Range(start_char, end_char)
            } else {
                // It's a single char
                CharacterClass::Single(start_char)
            };
            classes.push(ch_class);
            match string.first() {
                Some(b']') => {
                    string = &string[1..];
                    break;
                }
                Some(_) => continue,
                None => return Err((original_string, out)),
            }
        }
        out.push(AstNode::Characters(classes));
        Ok((string, out))
    } else {
        Err((original_string, out))
    }
}

fn node_repeat<'a, 'b>((mut string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    let original_string = string;
    let mut current_out = vec![];
    macro_rules! fail {
        () => {
            return Err((original_string, out));
        };
    }
    if string.first() == Some(&b'<') {
        string = &string[1..];
        string = parse_nodes(
            string,
            |string| !matches!(string.first(), Some(b':')),
            &mut current_out,
        );
        if string.first() != Some(&b':') {
            fail!();
        }
        string = &string[1..];
        let Some(end_index) = string.iter().position(|byte| *byte == b'>') else {
            fail!();
        };
        let Ok(repeat_params_string) = std::str::from_utf8(&string[..end_index]) else {
            // The parameters must be valid UTF-8
            fail!();
        };
        string = &string[(end_index + 1)..];
        let node =
            if let Some(comma_index) = repeat_params_string.bytes().position(|byte| byte == b',') {
                let (min_string, max_string) = repeat_params_string.split_at(comma_index);
                let Ok(min): std::result::Result<u32, _> = min_string.parse() else {
                    // number that cannot be parsed
                    fail!();
                };
                let Ok(max): std::result::Result<u32, _> = max_string[1..].parse() else {
                    // number that cannot be parsed
                    fail!();
                };
                AstNode::Repeat {
                    min,
                    max,
                    pattern: Pattern { nodes: current_out },
                }
            } else {
                let Ok(times): std::result::Result<u32, _> = repeat_params_string.parse() else {
                    // number that cannot be parsed
                    fail!();
                };
                AstNode::Repeat {
                    min: times,
                    max: times,
                    pattern: Pattern { nodes: current_out },
                }
            };
        out.push(node);
        Ok((string, out))
    } else {
        Err((original_string, out))
    }
}

fn get_utf8_char(string: &[u8]) -> Option<(char, &[u8])> {
    string
        .utf8_chunks()
        .next()
        .and_then(|chunk| chunk.valid().chars().next())
        .map(|ch: char| (ch, &string[ch.len_utf8()..]))
}

fn starts_at_path_component_boundary(string: &[u8]) -> bool {
    string.is_empty() || get_utf8_char(string).is_some_and(|(ch, _)| is_separator(ch))
}

fn node_cur_or_parent_dir<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    // We have to look behind and ahead to make sure this is an isolated node
    match out.last() {
        None | Some(AstNode::RootDir) | Some(AstNode::Separator) => match string {
            [b'.', b'.', next_string @ ..] if starts_at_path_component_boundary(next_string) => {
                out.push(AstNode::ParentDir);
                Ok((next_string, out))
            }
            [b'.', next_string @ ..] if starts_at_path_component_boundary(next_string) => {
                out.push(AstNode::CurDir);
                Ok((next_string, out))
            }
            _ => Err((string, out)),
        },
        _ => Err((string, out)),
    }
}

fn node_literal_string<'a, 'b>((string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    // Bytes that can start other nodes
    const MEANINGFUL_BYTES: &[u8] = b"*?[]{}<>,:/\\";
    // Take at least one byte, but if we find a meaningful byte, leave that alone for further parsing
    if let Some(index_of_meaningful_byte) = string[1..]
        .iter()
        .position(|byte| MEANINGFUL_BYTES.contains(byte))
        .map(|idx| idx + 1)
    {
        out.push(AstNode::LiteralString(
            string[0..index_of_meaningful_byte].into(),
        ));
        Ok((&string[index_of_meaningful_byte..], out))
    } else {
        out.push(AstNode::LiteralString(string.into()));
        Ok((b"", out))
    }
}
