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
    Characters {
        classes: Vec<CharacterClass>,
        negated: bool,
    },
    Alternatives {
        choices: Vec<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterClass {
    Single(char),
    Range(char, char),
}

/// Parse a pattern: nu-glob's syntax, plus `{a,b}` alternatives. `\` is a literal character, as
/// it is for nu-glob (a path separator on Windows).
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

/// Most patterns one [`expand_alternatives`] call may spell out.
const MAX_EXPANSIONS: usize = 1024;

/// The expansions of part of a pattern, and the rest of the pattern after it.
type Expanded<'a> = (Vec<Vec<u8>>, &'a [u8]);

/// Spell out `pattern` as nu-glob patterns, one for each combination of its `{a,b}` choices.
///
/// The braces follow the same rules as [`parse`]: a group needs a comma at its top level, and a
/// brace in a `[...]` class is a class member. Everything else is kept as written, so each
/// pattern means to nu-glob what that part of `pattern` means to dc-glob.
pub fn expand_alternatives(pattern: &str) -> anyhow::Result<Vec<String>> {
    let (expansions, _) = expand_sequence(pattern.as_bytes(), false)?;
    expansions
        .into_iter()
        .map(|bytes| Ok(String::from_utf8(bytes)?))
        .collect()
}

/// Expand `string` up to its end, or with `in_group` up to a top-level `,` or `}`. Returns the
/// expansions and the rest of `string`.
fn expand_sequence(string: &[u8], in_group: bool) -> anyhow::Result<Expanded<'_>> {
    let mut expansions = vec![vec![]];
    let mut rest = string;
    while let Some(&byte) = rest.first() {
        if in_group && matches!(byte, b',' | b'}') {
            break;
        }
        let taken = if byte == b'{'
            && let Some((choices, after)) = expand_group(&rest[1..])?
        {
            if expansions.len() * choices.len() > MAX_EXPANSIONS {
                anyhow::bail!("the pattern spells out more than {MAX_EXPANSIONS} alternatives");
            }
            expansions = expansions
                .iter()
                .flat_map(|prefix| {
                    choices
                        .iter()
                        .map(move |choice| [&prefix[..], choice].concat())
                })
                .collect();
            rest = after;
            continue;
        } else if byte == b'[' {
            // A whole class, or a lone `[`.
            let mut scratch = vec![];
            node_character_class((rest, &mut scratch))
                .map_or(1, |(after, _)| rest.len() - after.len())
        } else {
            1
        };
        for expansion in &mut expansions {
            expansion.extend_from_slice(&rest[..taken]);
        }
        rest = &rest[taken..];
    }
    Ok((expansions, rest))
}

/// Expand the group that starts after a `{`. Returns its expansions and the rest of `string`
/// after its `}`, or `None` when the group isn't closed, so the `{` is literal. A group without a
/// comma keeps its braces.
fn expand_group(string: &[u8]) -> anyhow::Result<Option<Expanded<'_>>> {
    let mut choices = vec![];
    let mut has_comma = false;
    let mut rest = string;
    loop {
        let (expansions, after) = expand_sequence(rest, true)?;
        choices.extend(expansions);
        if choices.len() > MAX_EXPANSIONS {
            anyhow::bail!("the pattern spells out more than {MAX_EXPANSIONS} alternatives");
        }
        match after.first() {
            Some(b',') => {
                has_comma = true;
                rest = &after[1..];
            }
            Some(b'}') => {
                rest = &after[1..];
                break;
            }
            _ => return Ok(None),
        }
    }
    if !has_comma {
        choices = choices
            .into_iter()
            .map(|choice| [&b"{"[..], &choice, b"}"].concat())
            .collect();
    }
    Ok(Some((choices, rest)))
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
        .or_else(node_cur_or_parent_dir)
        .or_else(node_literal_string)
    {
        Ok((remaining, _)) => remaining,
        Err((remaining, out)) => {
            if !remaining.is_empty() {
                push_literal(out, remaining);
            }
            b""
        }
    }
}

/// Push literal bytes, extending the previous node when it is also a literal.
///
/// A byte like `]` or `,` ends a literal so another node can try it, and is then taken as a
/// literal itself. Joining the pieces keeps a name like `a]c` one literal, which the globber
/// relies on to treat a literal as a whole path component.
fn push_literal(out: &mut Vec<AstNode>, bytes: &[u8]) {
    if let Some(AstNode::LiteralString(previous)) = out.last_mut() {
        previous.extend_from_slice(bytes);
    } else {
        out.push(AstNode::LiteralString(bytes.into()));
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

/// `{a,b}` alternatives. As in bash, zsh and fish, a group needs a comma at its top level to be
/// one: in `a{b}`, `{}` or `HEAD@{1}` the braces are literal, and any pattern syntax between
/// them still applies.
fn node_alternatives<'a, 'b>((mut string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    let original_string = string;
    if string.first() != Some(&b'{') {
        return Err((original_string, out));
    }
    string = &string[1..];
    let mut choices = vec![];
    let mut current_out = vec![];
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
                break;
            }
            Some(_) => continue,
            None => {
                return Err((original_string, out));
            }
        }
    }
    if choices.is_empty() {
        // No comma, so the braces are literal. Joining the literals keeps `a{b}` one literal.
        push_literal(out, b"{");
        for node in current_out {
            match node {
                AstNode::LiteralString(bytes) => push_literal(out, &bytes),
                node => out.push(node),
            }
        }
        push_literal(out, b"}");
    } else {
        choices.push(Pattern { nodes: current_out });
        out.push(AstNode::Alternatives { choices });
    }
    Ok((string, out))
}

/// A `[...]` class. `\` is a class member like any other character.
fn node_character_class<'a, 'b>((mut string, out): NodeInput<'a, 'b>) -> NodeResult<'a, 'b> {
    let original_string = string;
    if string.first() == Some(&b'[') {
        string = &string[1..];
        // Only a leading exclamation mark negates a character class.
        let negated = string.first() == Some(&b'!');
        if negated {
            string = &string[1..];
        }
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
        out.push(AstNode::Characters { classes, negated });
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
    const MEANINGFUL_BYTES: &[u8] = b"*?[]{},/\\";
    // Take at least one byte, but if we find a meaningful byte, leave that alone for further parsing
    if let Some(index_of_meaningful_byte) = string[1..]
        .iter()
        .position(|byte| MEANINGFUL_BYTES.contains(byte))
        .map(|idx| idx + 1)
    {
        push_literal(out, &string[0..index_of_meaningful_byte]);
        Ok((&string[index_of_meaningful_byte..], out))
    } else {
        push_literal(out, string);
        Ok((b"", out))
    }
}
