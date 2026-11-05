# 04 Blocks and pipelines

Files: `src/winnow/syntax/parser/lite_parser.rs`, `src/winnow/syntax/parser/parse_pipelines.rs`.

This layer corresponds to nu-parser's `lite_parser.rs`, `parse_pipelines.rs` and the top of its
`parse_block`. It takes a `Tokens` stream and produces `Block { pipelines }`, deciding where
statements start and end and which tokens belong to which command. It never looks inside an item,
and it knows nothing about keywords, values or operators. Grouping rules here are the ones nu-parser
applies before any signature is known.

The work splits as in nu-parser. `lite_parser.rs` groups tokens into commands and holds the newline
rules around `|`. `parse_pipelines.rs` drives it: `parse_block` walks the statements,
`parse_pipeline` collects and parses the commands of one pipeline, and `parse_pipeline_element`
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
2. **Predeclares definitions** with `parse_def_predecl` (below).
3. **Checks for a missing end**, once over the whole block (`missing_end`, nu's
   `last_non_comment_token`). If the last token outside a trailing run of comment lines is a `|`,
   the pipeline has no end. So `ls |`, `ls | # c` and `ls |\n# c` without a final newline are
   errors, while `ls |\n` and `ls |\n# c\n` are not. It fires whichever command absorbed the pipe.
   The block's last pipeline carries it as `LiteError::PipelineMissingEnd`, recovered.
4. **Loops over statements** in `parse_statements`, handling `Eol`, `;` and comments itself and
   calling `parse_pipeline` for anything else.
5. **Recovers per statement** (chapter 03).

The statement loop is a plain `while let` over `peek_token`, because each token's meaning depends on
the one before it: a blank line drops pending comments. `BlockStatements::parse` runs the same loop
a few statements at a time for the front end.

With a lookup that answers `defers_command_changing_blocks`, a nested block holding a statement that
changes which commands exist is left empty, for the lowering to parse again statement by statement.

## Comments

Comments attach while grouping:

* A comment on its own line is a **leading comment** of the next pipeline
  (`Pipeline::leading_comments`), unless a blank line separates them. A blank line right above a
  line-leading `|` doesn't count (`# doc\n\n| def f [] {}`), because nu's lexer turns the end of
  line before that `|` into it. For a `def`, leading comments are its documentation.
* A comment after the last token of a line is a **trailing comment** of that pipeline, as are
  comments between the elements of a multi-line pipeline.
* A comment after a `;`, or after a `|` that ends its pipeline, on that line, goes to the next
  statement. It is that statement's leading comment, past blank lines too, unless comment lines come
  right before the statement, which nu puts in its place (`def foo [] {}; # doc\ndef bar [] {}`
  documents `bar`).
* Every comment, attached or not, is in `Ast::comments`, in source order. Nested constructs record
  theirs through `working_set.add_comment`.

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

`parse_pipeline` parses `command (| command)*` in two passes, because a command parses differently
when it is one element of a longer pipeline (`Position`, chapter 05). The lite pass collects every
command's tokens, and then each one is parsed:

```text
loop:
  take any `|`s (a pipeline may start with one; `a | | b` is `a | b`)
    after each, after_pipe decides whether a command follows
  no `|` and a command already collected?
    the pipeline goes on only through take_pipe_on_later_line
  parse_lite_command; if an `e>|` ended it, after_pipe decides again
parse_pipeline_element for each command, with in_pipeline = more than one command
```

It returns `None` when there was no command at all, such as a lone `|` before a blank line.

An element keeps two pipes. `pipe` is the last `|` before it, which the formatter and the element's
span use. `lite_pipe` is the pipe nu's lite parser keeps with the command, which the lowering hands
on. nu sets `command.pipe` on the command it is reading and pushes the command only if it has parts,
so a command keeps the pipe that ended it, or else the last pipe read while no command was open,
even one from the statement before. `CarriedPipes` hands those on to the next statement.

### Newlines around a `|`

The rules are nu's and they are asymmetric. Both are combinators over `eol`, `comment` and `pipe`.

**After a pipe**, `after_pipe_lines` runs. It takes the comments on the pipe's line and, after a
plain `|` only, one end of line and any number of comment lines (`Comment* [Eol (Comment Eol)*]`).
It then reports what comes next: `AfterPipe::Command` when a command follows, and
`AfterPipe::Dangling` when a blank line, a `;` or the end of the block does. After an `e>|`, any end
of line makes it dangling. A dangling pipe closes the pipeline and is dropped. `after_pipe` records
the comments for `parse_pipeline`. The predeclaration scan calls `after_pipe_lines` itself, so both
decide by the same rule whether a pipe joins a command to its pipeline.

**After a command**, `take_pipe_on_later_line` looks forward. The pipeline continues only through
exactly `Eol (Comment Eol)* Pipe`, which is `pipe_on_later_line` under `opt`. When no `|` follows,
`opt` resets the stream, so the end of line and comment lines stay for `parse_block`. The comments
taken this way become trailing comments.

A backtrack is only a position, so trying this after every command costs a few token comparisons and
no allocation.

| Text | Pipelines | Why |
| --- | --- | --- |
| `a\n# c\n\| b` | one | a comment line doesn't close the pipeline |
| `a\n\n\| b` | two | a blank line does; the second starts with a dangling pipe that yields nothing |
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

## Commands: `LiteCommand`

`parse_lite_command(tokens, at_line_start)` collects one command's tokens without interpreting them:

```rust,ignore
pub struct LiteCommand {
    pub parts: Vec<Token>,              // the items (after `=`, the rest of the line)
    pub end: usize,                     // byte offset just past the last part
    pub attributes: Vec<Vec<Token>>,    // preceding @attribute lines
    pub redirections: Vec<(Spanned<RedirectionOperator>, Option<Token>)>,  // `o> file`, `e>|`
    pub pipe_after: Option<Span>,       // the `e>|` that ended the command
    pub comments: Vec<Comment>,
    pub lite_error: Option<Box<LiteError>>,   // the first error nu goes on past
    pub joins_next_line: bool,          // `^ls o>`: the next line's command joins
    pub lite_pipe: Option<Span>,
}
```

`lite_command.tokens(working_set)` gives the statement parsers a `Tokens` stream over `parts` that
ends at `end`.

The function is a loop over `peek_token`, like nu's lite parser, because once an assignment operator
has been seen the same tokens mean something else:

* **Items** go to `parts`.
* **An assignment operator** (`=`, `+=`, ...) switches to absorbing. Everything to the end of the
  line goes into `parts`, including pipes, redirections and comments, which is why `let x = ls |
  length` is a single command. A `|` at the end of the line, or at the start of the next line after
  any comment lines, continues absorption.
* **A redirection** (`lite_redirection`): a file redirection takes the next item as its target. A
  pipe redirection ends the command and acts as the pipe to the next element.
* **`Pipe`** ends the command, left for `parse_pipeline`.
* **`Eol`, `;`, `Eof`** end the command, not consumed.

### Lite-parse errors

nu's lite parser reports these and goes on, building the command the way the error leaves it, so
each is recovered: the command's `lite_error`, and the pipeline's first becomes
`Pipeline::lite_error`. The lowering reports a block's first before any of its statements' errors,
because nu lite-parses a whole block first (`lower::BlockErrors`).

| `LiteError` | Input | The command becomes |
| --- | --- | --- |
| `ShellOrOr` | `ls \|\| pwd` | `\|\|` is an item, passed to `ls` |
| `NothingToRedirect` | `o> f` | it and its target are items, so this is the external `o>`. After attribute lines it has a command. An `e>\|` with nothing before it ends a command nu drops |
| `MultipleRedirections` | a second redirection of a stream | it and its target are items, and the command has no redirection, because nu took the command's out before refusing |
| `MissingRedirectionTarget` | `^ls o>` | the operator is an item. An `=`, another redirection or a `\|\|` after it is an item too. An end of line ends the command but not the pipeline, so a command on the next line joins with no `\|` (`joins_next_line`) |
| `PipelineMissingEnd` | `ls \|` at the end of a block | see `parse_block` above |

### Attribute lines

`lite_attribute_lines` runs when `at_line_start` is set: the first command of a pipeline, and any
command that starts a line, even inside a pipeline. While the next item starts with `@`, every token
up to the end of the line or a `;` becomes an item word of one attribute, pipes and redirections
included. So `@search-terms a | b` hands `|` and `b` to the attribute. An `@foo` after a `|` on the
same line is an ordinary command head.

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
defined later in the same block. `parse_def_predecl`, in `parse_def.rs`, scans the tokens for `def`,
`export def`, `extern` and `export extern` at line starts and, when the name item reads as a string
and a signature item follows it, registers the name in the current scope with
`working_set.add_predecl` before any statement is parsed:

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
  attribute on to the `def` below, which still is. `after_pipe_lines` decides, so a `def` with a
  dangling pipe is a pipeline of its own and is predeclared.
* The same name predeclared twice in one block is "duplicate command definition within a block",
  recovered. The same name in a nested block is fine.
* An `alias` is never predeclared. `parse_alias` declares it once its statement is parsed, so `g 1`
  before `alias g = echo` calls an unknown, external `g`, and `def foo` plus `alias foo` is fine.
* The scan also records where each `type` statement is (`TypeAliasPredecl`), so an engine can tell a
  use above the definition from an unknown type.

Scopes are entered and left by the constructs that create them: `parse_subexpression`,
`parse_block_body_unchecked` and `parse_closure_parts`. A module body is a block body, parsed with
`parse_block_body_unchecked`.
