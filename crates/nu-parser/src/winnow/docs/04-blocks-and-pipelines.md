# 04 Blocks and pipelines

Files: `src/winnow/syntax/parser/lite_parser.rs`, `src/winnow/syntax/parser/parse_pipelines.rs`.

This layer corresponds to nu-parser's `lite_parser.rs`, `parse_pipelines.rs` and the top of its
`parse_block`. It takes a `Tokens` stream and produces `Block { pipelines }`, deciding where
statements start and end and which tokens belong to which command. It never looks inside an item,
and it knows nothing about keywords, values or operators. Grouping rules here are the ones nu-parser
applies before any signature is known.

The work splits as in nu-parser. `lite_parser.rs` groups a block's tokens into pipelines of commands,
once for the block, as a `LiteBlock`. `parse_pipelines.rs` reads it: `parse_block` predeclares from it
and walks its pipelines, `parse_pipeline` parses the commands of one, and `parse_pipeline_element`
turns one command into an expression and its redirection.

## `parse_block`

```rust,ignore
pub fn parse_block<'a>(tokens: Tokens<'_, 'a>, span: Span) -> Block<'a>
```

Every block goes through this function: the file, closure and block bodies, subexpressions, `let`
and assignment values, `match` bodies written as blocks. The block's `span` covers its contents, so
for a `{ ... }` body it is the text between the braces, which a formatter needs to re-indent. In
order, it:

1. **Takes the block's lexer error.** `WorkingSet::first_lex_error_in` finds the first error nu's
   lexer went past within the block's text, recorded as recovered and carried by the first pipeline
   as `Block::lex_error`. A closure takes the first in its whole inside, parameters included
   (`parse_block_lexed_from`).
2. **Lite-parses the block** with `LiteBlock::parse`, which everything below reads.
3. **Takes a missing end** (`LiteBlock::missing_end`, nu's `last_non_comment_token`). If the last token
   outside a trailing run of comment lines is a `|`, the pipeline has no end. So `ls |`, `ls | # c`
   and `ls |\n# c` without a final newline are errors, while `ls |\n` and `ls |\n# c\n` are not. It
   fires whichever command absorbed the pipe. The block's last pipeline carries it as
   `LiteError::PipelineMissingEnd`, recovered.
4. **Predeclares definitions** with `parse_def_predecl` (below).
5. **Parses each pipeline** in `parse_statements`, which calls `parse_pipeline`.
6. **Recovers per statement** (chapter 03).

`BlockStatements::parse` runs the same loop a few statements at a time for the front end, and holds
its place between calls as the index of the next command.

With a lookup that answers `defers_command_changing_blocks`, a nested block holding a statement that
changes which commands exist is left empty, for the lowering to parse again statement by statement.

## Comments

Each command keeps the comments nu's lite parser gives it (`LiteBlock::comments`):

* Comment lines right before a command wait for it, and replace any comment it has when it gets an
  item that is no attribute. A blank line drops them. A blank line right above a line-leading `|`
  doesn't count (`# doc\n\n| def f [] {}`), because nu's lexer turns the end of line before that `|`
  into it.
* A comment beside something goes to the command being read and drops the comment lines waiting. After
  a `;`, or after a `|` that ends its pipeline, that is the next command, past blank lines too
  (`def foo [] {}; # doc\ndef bar [] {}` documents `bar`).

A pipeline's `leading_comments` are those of its first command before its first item, and its
`trailing_comments` are the rest of its commands' comments. For a `def`, the two together are its
documentation, as nu's list is. A comment nu's lite parser drops is in no pipeline. Every comment,
kept or not, is in `Ast::comments`, in source order.

```rust
use nu_parser::winnow::syntax::parse;

let src = "# doc one\n# doc two\ndef foo [] { }\n\n# detached\n\nls # trailing\n";
let ast = parse(src).unwrap();
let def = &ast.block.pipelines[0];
assert_eq!(def.leading_comments.len(), 2);
assert_eq!(def.leading_comments[0].body(src), "doc one");
let ls = &ast.block.pipelines[1];
assert!(ls.leading_comments.is_empty());
assert_eq!(ls.trailing_comments.len(), 1);
assert_eq!(ast.comments.len(), 4);
```

## Pipelines

`parse_pipeline` parses the commands of one pipeline, which the lite parse has found already, because
a command parses differently when it is one element of a longer pipeline (`Position`, chapter 05). It
calls `parse_pipeline_element` for each, with `in_pipeline` set when there is more than one.

An element keeps two pipes. `pipe` is the last `|` before it, which the formatter and the element's
span use. `lite_pipe` is the pipe nu's lite parser keeps with the command, which the lowering hands
on. nu sets `command.pipe` on the command it is reading and pushes the command only if it has parts,
so a command keeps the pipe that ended it, or else the last pipe read while no command was open, even
one from the statement before. A `|` after a pipeline's last command is its `dangling_pipe`.

### Newlines around a `|`

The rules are nu's and they are asymmetric. They come from two places.

**nu's lexer folds a line end into a `|`.** An end of line that only comment lines stand between and a
`|` starting a later line is dropped (`Eol (Comment Eol)* Pipe`). The winnow lexer keeps those line
ends, so `LiteBlock::parse` skips them, walking a long run of comment lines once. `&&`, `o>|` and
`2>&1` are `Pipe` tokens too, but nu's lexer reads them apart, and they fold nothing.
`LiteBlock::parse_folded` is for tokens with those line ends dropped already, which the lowering's
`lex_span` gives.

**nu's lite parser keeps a pipeline open after a `|`.** An end of line ends the command and its
pipeline, unless the last token before it, comment lines skipped, is a `|`
(`last_non_comment_token`). After an `e>|` it isn't, so the pipeline goes on to a later line only
through a folded `|`.

| Text | Pipelines | Why |
| --- | --- | --- |
| `a\n# c\n\| b` | one | a comment line doesn't close the pipeline |
| `a\n\n\| b` | two | a blank line does; the `\|` is kept with `b` |
| `a \|\n\n\| b` | one | nu's lexer folds the end of line before a line-leading `\|` into it, so after a `\|` one blank line is allowed |
| `a \|\n\n\n\| b` | two | two blank lines are not |
| `a \|\n\n b` | two | the pipe dangles |
| `a e>\|\n# c\n\| b` | one | a line-leading `\|` still continues |
| `a e>\|\nb` | two | after a redirection pipe, any end of line dangles |
| `ls e>\|; ls` | two | reached only after a redirection pipe: after a plain `\|` the lexer already refused the `;` |

```rust
use nu_parser::winnow::syntax::parse;

let two = parse("[1 2 3]\n\n| length\n").unwrap();     // a blank line closes the pipeline
assert_eq!(two.block.pipelines.len(), 2);
let one = parse("[1 2 3]\n# c\n| length\n").unwrap();    // a comment line does not
assert_eq!(one.block.pipelines.len(), 1);
let one = parse("[1 2 3] |\n\n| length\n").unwrap();   // nor one between a `|` ending a line and one starting a line
assert_eq!(one.block.pipelines.len(), 1);
assert!(parse("ls |\n# c").is_err());                     // no command after the pipe
assert!(parse("ls |\n# c\n").is_ok());                    // the final newline makes it a dangling pipe
```

## Commands: `LiteBlock`

`LiteBlock::parse(source, tokens)` groups a block's tokens without interpreting them. It holds each
kind of list once for the whole block, and a command is ranges of them, so that reading a block costs
a few allocations, not a few per command. Every block is read this way, the file's before its second
thread starts.

```rust,ignore
pub struct LiteBlock {
    parts: Vec<Token>,                 // every command's items, in order
    commands: Vec<LiteBlockCommand>,   // ranges of these lists, its pipes, what ended it
    comments: Vec<Span>,
    attribute_ends: Vec<u32>,          // where each attribute line's parts end
    redirections: Vec<LiteRedirection>,  // `o> file`, `e>|`
    errors: Vec<(u32, LiteError)>,     // with the command being read then
    missing_end: Option<LiteError>,    // a `|` ending the block
}
```

`pipelines()` gives the commands of each pipeline. `command(command, tokens)` gives the statement
reader a `LiteCommand`, whose `tokens(working_set)` is a `Tokens` stream over the command's items.

The function is nu's `lite_parse`: a loop over the tokens, because what a token means depends on the
token before it and on the mode:

* **Items** go to the command's parts.
* **An assignment operator** (`=`, `+=`, ...) switches to absorbing. Everything to the end of the
  line goes into the parts, including pipes and redirections, which is why `let x = ls | length` is a
  single command. A `|` at the end of the line, or at the start of the next line after any comment
  lines, continues absorption.
* **A redirection**: a file redirection takes the next item as its target. A pipe redirection ends the
  command and acts as the pipe to the next element.
* **`Pipe`** ends the command.
* **`Eol`** ends the command and its pipeline, as above. **`;`** ends both.

### Lite-parse errors

nu's lite parser reports these and goes on, building the command the way the error leaves it, so
each is recovered. A pipeline's first becomes `Pipeline::lite_error`, and one after the block's last
command, which no pipeline carries, is `Block::lite_error_without_pipelines`. The lowering reports a
block's first before any of its statements' errors, because nu lite-parses a whole block first
(`lower::BlockErrors`).

| `LiteError` | Input | The command becomes |
| --- | --- | --- |
| `ShellOrOr` | `ls \|\| pwd` | `\|\|` is an item, passed to `ls` |
| `NothingToRedirect` | `o> f` | it and its target are items, so this is the external `o>`. After attribute lines it has a command. An `e>\|` with nothing before it ends a command nu drops |
| `MultipleRedirections` | a second redirection of a stream | it and its target are items, and the command has no redirection, because nu took the command's out before refusing |
| `MissingRedirectionTarget` | `^ls o>` | the operator is an item. An `=`, another redirection or a `\|\|` after it is an item too. An end of line ends the command but not the pipeline, so a command on the next line joins with no `\|` |
| `PipelineMissingEnd` | `ls \|` at the end of a block | see `parse_block` above |

### Attribute lines

An item starting with `@` right after an end of line, a `;` or the start of the block starts an
attribute line, even inside a pipeline. Every token up to the end of the line or a `;` becomes an item
word of one attribute, pipes and redirections included. So `@search-terms a | b` hands `|` and `b` to
the attribute. An `@foo` after a `|` on the same line is an ordinary command head. The lines stay among
the command's parts, as nu has them: `attribute_ends` tells where each ends, and `command_parts` is
what comes after them.

A `|` starting the next line continues the attribute line, because nu's lexer turns the end of line
before it into that `|`: `@search-terms a\n| b` gives the attribute `a`, `|` and `b`.

The next attribute or the definition must start on the very next line. A blank or comment-only line
in between ends the command with no definition, an error. A trailing comment on the attribute line
is fine, and a comment line goes to the statement after it. The exceptions are a `|` line after one
blank line and any comment lines, and a `|` line right after an attribute line that a `;` ends: the
attribute line ends at its own end of line and the `|` continues the pipeline the attribute lines
start, with no command of their own (`@a x`, a blank line, `| y` pipes the external `@a x` into
`y`).

`lite_parse_parts` is nu's lite parse of the tokens inside `[...]`, which chapter 06 describes.

## Elements: `parse_pipeline_element`

`parse_pipeline_element(working_set, lite_command, in_pipeline)` turns one `LiteCommand` into the
element's expression and redirection:

* The items go to `parse_expression` with `Position::Element` when `in_pipeline` is set, else
  `Position::Statement`.
* Attribute lines go to `parse_attribute`, and the command after them must be a `def` or `extern`,
  plain or exported, read with `parse_builtin_commands`, giving an `Expr::AttributeBlock`. In an
  element of a longer pipeline they aren't attributes: the parser reads such an element from all its
  items, the attribute lines' first, so its head is the first attribute word (`ls |\n@search-terms
  foo\ndef bar [] {}` has a second element headed by `@search-terms`).
* `parse_redirection` builds the `PipelineRedirection`: one stream (`Single`), or stdout and stderr
  separately (`Separate`). The target of a file redirection goes through `parse_value`.
* `rejects_redirection` refuses a redirection on the statements listed in chapter 05, in a pipeline
  of one command. `drops_redirection` covers the ones nu ignores, recorded in `Ast::ignored`.

## Predeclaration

Nushell resolves command names by longest match against known declarations, including commands
defined later in the same block. `parse_def_predecl`, in `parse_def.rs`, reads the lite block as nu's
`parse_block` does. In each pipeline of one command it looks at the command's parts after its attribute
lines, at least three of them, for `def`, `export def`, `extern` and `export extern`. When the name
item reads as a string and a signature item follows it, it registers the name in the current scope
with `working_set.add_predecl` before any statement is parsed:

```rust
use nu_parser::winnow::syntax::{parse, ast::Expr};

let src = "my cmd 1\ndef \"my cmd\" [x] { $x }";
let ast = parse(src).unwrap();
match &ast.block.pipelines[0].elements[0].expr.expr {
    Expr::Call(call) => {
        assert_eq!(call.head.name, "my cmd");   // resolved although defined below
        assert_eq!(call.arguments.len(), 1);
    }
    other => panic!("{other:?}"),
}
```

The rules:

* `parse_definition_name` unquotes the name as nu's `parse_string` does: `def "'x'" [] {}` declares
  `'x'`, and `def "a"b [] {}` declares nothing.
* Only a pipeline of one command counts. A `def` that a `|` joins to another command isn't
  predeclared, on its line, on a later line, or after a `|` that ends the line before. Neither is
  one after an attribute line inside a pipeline. A `|` ending an attribute line carries the
  attribute on to the `def` below, which still is. A `def` with a dangling pipe is a pipeline of its
  own and is predeclared.
* The same name predeclared twice in one block is "duplicate command definition within a block",
  recovered. The same name in a nested block is fine.
* An `alias` is never predeclared. `parse_alias` declares it once its statement is parsed, so `g 1`
  before `alias g = echo` calls an unknown, external `g`, and `def foo` plus `alias foo` is fine.
* It also records where each `type` statement is (`TypeAliasPredecl`), so an engine can tell a
  use above the definition from an unknown type.

Scopes are entered and left by the constructs that create them: `parse_subexpression`,
`parse_block_body_unchecked` and `parse_closure_parts`. A module body is a block body, parsed with
`parse_block_body_unchecked`.
