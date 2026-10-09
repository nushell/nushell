# 04 Blocks and pipelines (`src/parser/lite_parser.rs`, `src/parser/parse_pipelines.rs`)

This layer corresponds to nu-parser's `lite_parser.rs` and
`parse_pipelines.rs`, plus the top of its `parse_block`. It receives a
`Tokens` stream over lexed tokens (chapter 03) and produces
`Block { pipelines }`, deciding where statements start and end and which
tokens belong to which command. It never looks inside an item.

The work is split the way nu-parser splits it. `lite_parser.rs` groups
tokens into commands (`LiteCommand`, `parse_lite_command`) and holds the
newline rules around `|` (`after_pipe`, `take_pipe_on_later_line`).
`parse_pipelines.rs` drives it: `parse_block` walks the statements,
`parse_pipeline` collects and parses the commands of one pipeline, and
`parse_pipeline_element` turns one command into an expression and its
redirection.

## Entry point

```rust,ignore
pub fn parse_block<'a>(mut tokens: Tokens<'_, 'a>, span: Span) -> Block<'a>
```

Every block in the language goes through this function: the file, closure and
block bodies, subexpressions, `let`/assignment values, `match` bodies written
as blocks. It:

1. runs `parse_def_predecl` over all the tokens (below),
2. runs nu's `last_non_comment_token` check once over the whole block: if the
   last token that is not part of a trailing run of comment lines is a `|`,
   the block has a pipeline with no end and "command after `|`" is reported
   at that pipe. This is what makes `ls |`, `ls | # c` and `ls |\n# c` (no
   final newline) errors while `ls |\n` and `ls |\n# c\n` are not, and it
   fires whichever command absorbed the pipe (`alias x = ls |` too),
3. loops over tokens, handling `Eol`, `;` and comments itself and calling
   `parse_pipeline` for anything else,
4. recovers from errors per pipeline (chapter 03): the error goes to
   `working_set.error`, the stream is moved back to the start of the
   pipeline with `reset_to`, `skip_to_statement_end` skips to the next `Eol`
   or `;`, and a `garbage_pipeline` stands in for the statement.

The statement loop is a plain `while let` over `peek_token`, not a
combinator: each token's meaning depends on the one before it (a blank line
drops pending comments), and the loop says that more directly than a chain of
parsers would. A `;` after a `|` with only newlines and comments between is
refused earlier, by the lexer (nu's `is_complete`, chapter 02), so
`ls |\n\n; x` never reaches it.

The block's `span` is the span of its *contents*: for a `{ ... }` body that is
the text between the braces, which is what a formatter needs to re-indent.

## Comments

Comments are attached while grouping, following nu-parser's rules:

* A comment on its own line is a **leading comment** of the next pipeline
  (`Pipeline::leading_comments`), unless a blank line separates them, in
  which case it is dropped from the attachment (it stays in `Ast::comments`).
  For a `def`, leading comments are its documentation.
* A comment after the last token of a line is a **trailing comment** of that
  pipeline (`Pipeline::trailing_comments`), as are comments between the
  elements of a multi-line pipeline.
* Every comment, attached or not, is recorded in `Ast::comments` (sorted, in
  source order). Nested constructs record theirs through
  `working_set.add_comment`.

```rust
use nu_winnow_parser::parse;

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

`parse_pipeline` parses `command (| command)*` in two passes, because a
command is parsed differently when it is one element of a longer pipeline
(chapter 05, `Position`): first the lite pass collects every command's
tokens, then each one is parsed.

```rust,ignore
fn parse_pipeline<'a>(tokens: &mut Tokens<'_, 'a>, leading_comments: Vec<Comment>)
    -> ParseResult<Option<Pipeline<'a>>>
{
    let mut lite_commands: Vec<(Option<Span>, LiteCommand)> = Vec::new();   // (the `|` before it, the command)
    let mut trailing_comments = Vec::new();
    let mut pipe: Option<Span> = None;
    'commands: loop {
        // A pipeline may start with `|` (`( | str join)`) and `a | | b` is `a | b`.
        while let Some(token) = tokens.peek_token().filter(|token| token.contents == TokenContents::Pipe) {
            pipe = Some(token.span);
            tokens.next_token();
            if let AfterPipe::Dangling = after_pipe(tokens, &mut trailing_comments)? {
                break 'commands;
            }
        }
        if pipe.is_none() && !lite_commands.is_empty() {
            // After a command the pipeline goes on only through a `|` on a later line.
            if !take_pipe_on_later_line(tokens, &mut trailing_comments)? { break; }
            continue;
        }
        let lite_command = parse_lite_command(tokens, pipe.is_none())?;   // attributes only before the first command
        trailing_comments.extend(lite_command.comments.iter().copied());  // comments between its tokens
        let pipe_after = lite_command.pipe_after;                          // an `e>|` ended the command
        lite_commands.push((pipe.take(), lite_command));
        if pipe_after.is_some() {
            pipe = pipe_after;
            if let AfterPipe::Dangling = after_pipe(tokens, &mut trailing_comments)? {
                break;
            }
        }
    }
    if lite_commands.is_empty() { return Ok(None); }                     // only pipes: nu drops the empty command
    let single = lite_commands.len() == 1;
    for (pipe, lite_command) in &lite_commands {
        let (expr, redirection) = parse_pipeline_element(working_set, lite_command, !single)?;
        elements.push(PipelineElement { span, pipe: *pipe, expr, redirection });
    }
    let terminator = /* a following `;` */;
    Ok(Some(Pipeline { span, elements, leading_comments, trailing_comments, terminator }))
}
```

The rules for newlines around a `|` are nu's lite parser's exactly, and they
are asymmetric. Both are written as winnow combinators over the token parsers
`eol`, `comment` and `pipe` from `tokens.rs`, each of which matches one token
of its kind or backtracks:

* `after_pipe` runs after a `|` has been consumed. It takes comments on the
  same line, then *one* end of line and any number of comment lines
  (`Comment* [Eol (Comment Eol)*]`):

  ```rust,ignore
  let same_line: Vec<Token> = repeat(0.., comment).parse_next(tokens)?;
  let later_lines: Option<Vec<Token>> =
      opt(preceded(eol, repeat(0.., terminated(comment, eol)))).parse_next(tokens)?;
  ```

  and then reports what comes next: `AfterPipe::Command` when a command
  follows, `AfterPipe::Dangling` when a blank line, a `;` or the end of the
  block does. A dangling `|` closes the pipeline and is dropped silently, as
  nu drops it: `a |\n\n b` is two pipelines. The `;` case is reached only
  after a redirection pipe (`ls e>|; ls` is two pipelines, as in nu): after a
  plain `|` the lexer has already refused the `;`.
* `take_pipe_on_later_line` looks *forward* from the end of a command: the
  pipeline continues only through exactly `Eol (Comment Eol)* Pipe`, that is
  one end of line, comment lines each on their own line, and the pipe, which
  is left for `parse_pipeline` to consume. The sequence is
  `pipe_on_later_line`:

  ```rust,ignore
  fn pipe_on_later_line(tokens: &mut Tokens<'_, '_>) -> ParseResult<Vec<Token>> {
      preceded(eol, terminated(repeat(0.., terminated(comment, eol)), peek(pipe))).parse_next(tokens)
  }
  ```

  `take_pipe_on_later_line` runs it under `opt`. When no `|` follows, `opt`
  resets the stream to where it started, so the end of line and the comment
  lines are left for `parse_block`. A blank line in between closes the
  pipeline instead (`a\n\n| b` is two pipelines, the second starting with a
  dangling pipe that yields nothing). The comments taken this way become
  trailing comments. This is the leading-pipe style:

  ```text
  ls
  # a comment
  | length
  ```

A backtrack is only a position (`ParseFailure::NoMatch`, chapter 03), so
trying `pipe_on_later_line` after every command costs a few token
comparisons and no allocation.

`parse_pipeline` returns `None` for the pipeline when there was no command at
all (a lone `|` before a blank line). A `|` that only comments follow at the very end of a block is reported
once, by `parse_block` (above), not by `after_pipe`.

```rust
use nu_winnow_parser::parse;

let two = parse("[1 2 3]\n\n| length\n").unwrap();     // a blank line closes the pipeline
assert_eq!(two.block.pipelines.len(), 2);
let one = parse("[1 2 3]\n# c\n| length\n").unwrap();    // a comment line does not
assert_eq!(one.block.pipelines.len(), 1);
assert!(parse("ls |\n# c").is_err());                     // no command after the pipe
assert!(parse("ls |\n# c\n").is_ok());                    // the final newline makes it a dangling pipe
```

## Commands: `LiteCommand`

`parse_lite_command(tokens, first)` collects one command's tokens without
interpreting them; `first` is set for the first command of a pipeline, the
only place attribute lines can precede it:

```rust,ignore
pub struct LiteCommand {
    pub parts: Vec<Token>,                                                 // the items (after `=`, the rest of the line)
    pub end: usize,                                                        // byte offset just past the last part
    pub attributes: Vec<Vec<Token>>,                                       // preceding @attribute lines
    pub redirections: Vec<(Spanned<RedirectionOperator>, Option<Token>)>,  // `o> file`, `e>|`
    pub pipe_after: Option<Span>,                                          // the `e>|` that ended the command
    pub comments: Vec<Comment>,
}
```

`lite_command.tokens(working_set)` gives the statement parsers a `Tokens`
stream over `parts` that ends at `end`, so "expected block" after `if $x` has
a position.

`parse_lite_command` is a loop over `peek_token`, as nu's lite parser is a
state machine: once an assignment operator has been seen, the same tokens
mean something else. The rules encoded in its loop:

* **Items** are pushed to `parts`.
* **`AssignmentOperator` token** (`=`, `+=`, ...): switch to *absorbing*
  mode. Everything up to the end of the line, including pipes, redirections
  and comments, is pushed into `parts`. This is why `let x = ls | length` and
  `$x = ls | length` are single commands whose right-hand side is a whole
  pipeline. A `|` at the end of the line, or a `|` at the start of the next
  line after any comment lines (`take_pipe_on_later_line` again), continues
  absorption.
* **`Redirection` token**: a file redirection takes the next item as its
  target; a pipe redirection (`e>|`, `o+e>|`) ends the command and acts as
  the pipe to the next element. A redirection with no command before it is
  an error.
* **`Pipe`** ends the command and is left for `parse_pipeline` to consume.
  **`PipePipe`** is a `ShellSyntax` error (use `or`).
* **`Eol`**, **`;`**, **`Eof`** end the command (they are not consumed here).
* **Attribute lines** (`lite_attribute_lines`, first command only): while
  the next item starts with `@`, every token up to the end of the line or a
  `;` becomes an item word of one attribute, pipes and redirections included
  (`@search-terms a | b` hands `|` and `b` to the attribute, as nu does; an
  `@foo` after a `|` is an ordinary command head). The next attribute or the
  definition must start on the very next line; a blank or comment-only line
  in between is an error, as in nu (a trailing comment on the attribute line
  is fine).

The same file holds `lite_parse_parts`, nu's lite parse of the tokens inside
`[...]`. The list and table-row parsers run it over a bracket interior as nu
does: the items of every `|`-separated group become list items, and a
redirection and its target are dropped (chapter 06).

## Elements: `parse_pipeline_element`

`parse_pipeline_element(working_set, lite_command, in_pipeline)` turns one
`LiteCommand` into the element's expression and redirection:

* The command's items go to `parse_expression` (chapter 05) with
  `Position::Element` when `in_pipeline` is set and `Position::Statement`
  when the command is the whole pipeline.
* Attribute lines are parsed with `parse_attribute` (chapter 05), and the
  command after them must be a `def` or `extern` (plain or exported), parsed
  with `parse_builtin_commands`; the result is an `Expr::AttributeBlock`.
* `parse_redirection` builds the `PipelineRedirection` from
  `lite_command.redirections`: one stream (`Single`), or stdout and stderr
  separately (`Separate`); a second redirection of the same stream is an
  error. The target of a file redirection is parsed with `parse_value`.
* `rejects_redirection` (nu's `redirecting_builtin_error`) refuses a
  redirection on the statements listed in chapter 05. A redirection on
  `export-env` is dropped and recorded in `Ast::ignored`.

## Predeclaration

Nushell resolves command names by longest match against known declarations,
including commands defined *later* in the same block. `parse_def_predecl`
(in `parse_def.rs`, as in nu-parser) scans the tokens for `def`,
`export def`, `extern` and `export extern` at line starts and, when a
signature item (`[` or `(`) follows the name, registers the name in the
current scope with `working_set.add_predecl` before any statement is
parsed. As in nu, only a pipeline of one command counts: a `def` that a `|`
joins to another command (on its line, on a later line, or after a `|` that
ends the line before) is not predeclared, while a `|` ending an attribute
line carries the attribute on to the `def` below, which still is:

```rust
use nu_winnow_parser::{parse, ast::Expr};

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

`parse_def_predecl` also carries nu's duplicate check: the same name
predeclared twice in one block is "duplicate command definition within a
block", while the same name in a nested block is fine. An `alias` is never
predeclared, as in nu: `parse_alias` declares it once its statement is parsed
(`working_set.add_alias`), so `g 1` before `alias g = echo` calls an unknown,
external `g`, and `def foo` plus `alias foo` is fine. A call to an alias of an
external command is an external call (chapter 05).

Scopes are entered and left with `working_set.enter_scope()` and
`working_set.exit_scope()` by the constructs that create them: subexpressions,
block bodies and closures in `src/parser/parse_expressions.rs`
(`parse_subexpression`, `parse_block_body_unchecked`, `parse_closure_parts`)
and module bodies in `src/parser/parse_module.rs` (`parse_module`).

## What this layer does not do

It does not know keywords, values or operators. All of that starts in
`parse_expression` (chapter 05), which receives the `Tokens` stream of one
`LiteCommand` and its `Position`, and returns the element's expression.
Keep this separation: grouping rules here are the ones nu-parser applies
before any signature is known, and they must stay independent of what the
items mean.
