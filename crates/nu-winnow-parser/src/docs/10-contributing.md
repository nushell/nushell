# 10 Contributing: recipes and pitfalls

## Before you change the grammar

1. Find the reference behaviour. `nu -n -c '...'` on the local binary answers
   most questions in seconds; `crates/nu-parser/src/*.rs` in the Nushell
   repository answers the rest. Write down what nu does, including the error
   cases, and note which nu-parser function does it.
2. Decide the layer, and with it the file. The files are named after
   nu-parser's, so the nu-parser function you found in step 1 usually tells
   you where to look: its namesake file here holds the function of the same
   name.

   | The change is about | Edit | nu-parser file |
   | --- | --- | --- |
   | where an item ends, quoting, brackets, token kinds | `src/lex.rs` | `lex.rs` |
   | where commands and pipelines end, comments around `\|`, redirections, attribute lines | `src/parser/lite_parser.rs` | `lite_parser.rs` |
   | blocks, pipeline elements, what may be redirected | `src/parser/parse_pipelines.rs` | `parse_pipelines.rs` |
   | the dispatch of a command (statement, shorthand, assignment, math, call), operators and precedence, what one item means (`parse_value`), lists, records, tables, blocks, closures, match blocks | `src/parser/parse_expressions.rs` | `parse_expressions.rs` |
   | calls, flags and arguments, multi-word command names, external and `%` calls, attributes, the fixed signatures of nu's keyword commands that are calls here (`KeywordSignature`, `check_call`) | `src/parser/parse_calls.rs` | `parse_calls.rs` |
   | the keyword tables, `KeywordCall` (nu's flags of a keyword command) | `src/parser/parse_keywords.rs` | `parse_keywords.rs` |
   | `def`, `extern`, `for`, predeclaration | `src/parser/parse_def.rs` | `parse_def.rs` |
   | `let`, `mut`, `const` | `src/parser/parse_bindings.rs` | `parse_bindings.rs` |
   | `alias` | `src/parser/parse_alias.rs` | `parse_alias.rs` |
   | `module`, `use`, `export`, `export-env` | `src/parser/parse_module.rs` | `parse_module.rs` |
   | `where` | `src/parser/parse_source.rs` | `parse_source.rs` |
   | `if`, `match`, `while`, `loop`, `try`, `return`, `break`, `continue` | `src/parser/parse_control_flow.rs` | none: ordinary commands in nu |
   | parameter lists, variable declarations, input/output types | `src/parser/parse_signatures.rs` | `parse_signatures.rs` |
   | type annotations and completers | `src/parser/parse_shape_specs.rs` | `parse_shape_specs.rs` |
   | `match` patterns | `src/parser/parse_patterns.rs` | `parse_patterns.rs` |
   | the text of one literal: numbers, units, datetimes, binary, strings, interpolation, variables, cell paths, ranges | `src/parser/parse_literals.rs` | `parse_literals.rs` |
   | small helpers shared by several files | `src/parser/parse_helpers.rs` | `parse_helpers.rs` |
   | the token stream and the token parsers | `src/parser/tokens.rs` | none |
   | what every parser shares: source, known commands, collected comments and errors | `src/parser/working_set.rs` | `StateWorkingSet` (nu-protocol) |

3. Add the test in `tests/syntax.rs` first; it should fail.
4. Implement, then run `cargo test`, `cargo clippy --all-targets
   --all-features`, `cargo fmt`, the comparison scripts if the lexer or
   `parse_value` changed, and the benchmarks (see [Performance](#performance))
   if you touched a hot path or rewrote a parser with combinators.

## Conventions

* **Names follow nu-parser and nu-protocol.** A function that does the job of
  a nu-parser function has its name (`parse_value`, `parse_call`,
  `parse_def_predecl`, `find_longest_decl`) and lives in the file named like
  the nu-parser file that holds it. AST types take nu-protocol's names (`Expression` and `Expr`,
  `Argument::Named`, `PipelineRedirection`, `MatchPattern`, `SyntaxShape`),
  and the `WorkingSet` takes `StateWorkingSet`'s method names
  (`get_span_contents`, `error`, `find_decl`, `add_predecl`, `enter_scope`,
  `exit_scope`). Where nu has no counterpart, name the function in the same
  style (`parse_while`, `parse_match_arm`).
* **Descriptive variable names.** `working_set`, `tokens`, `token`,
  `keyword`, `expression`, `operator_token`, not `st`, `c`, `tok`, `kw`, `e`.
* **Parsers of one item take the working set first**, then the item's span,
  like nu's `parse_value(working_set, span, shape)`:
  `fn parse_record<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>>`.
  They read the text with `working_set.get_span_contents(span)`; a bracketed
  item is lexed again from its interior (`delimited_interior`, then `lex`)
  and its tokens parsed as a stream.
* **Parsers of a sequence read a `Tokens` stream** (`src/parser/tokens.rs`),
  which carries the working set as `tokens.working_set`. A statement parser
  takes the stream of its command's items, `fn parse_def<'a>(tokens:
  Tokens<'_, 'a>)`; the parts it is made of are winnow parsers,
  `fn(&mut Tokens<'_, 'a>) -> ParseResult<T>`, written with winnow's
  combinators (`repeat`, `opt`, `alt`, `preceded`, `terminated`,
  `repeat_till`, `expression`, `.verify_map`, ...) and the token parsers of
  `tokens.rs`: `item`, `keyword(word)`, `pipe`, `eol` and `comment` match one
  token or backtrack; `expected(what, parser)` and `cut_with(parser, error)`
  commit to a parser; `repeat_to_end(parser)` repeats one to the end of the
  stream; `tokens_until(word)` takes the tokens up to a keyword as a stream of
  their own. A match arm reads like its grammar:

  ```rust,ignore
  /// `pattern ( | pattern )* [if guard...] => body`.
  fn parse_match_arm<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<MatchArm<'a>> {
      let pattern = parse_or_pattern(tokens)?;
      let guard = opt(parse_match_guard).parse_next(tokens)?;
      let arrow = expected("`=>`", keyword("=>")).parse_next(tokens)?;
      let body = parse_match_arm_body(tokens)?;
      Ok(MatchArm { span: pattern.span.merge(body.span), pattern, guard, arrow: arrow.span, body })
  }
  ```

  A few parsers stay loops on purpose, because they mirror a state machine
  of nu-parser or read more clearly that way: `parse_block`'s statement loop,
  `parse_lite_command`, `lite_parse_parts`, `parse_parameters` (nu's
  `ParseMode`), `parse_interpolation_parts`, `parse_external_string`,
  `find_range_operators` and the lexer's `item_length`.
* **Errors are cuts; backtracks carry no diagnostic.** Every parser returns
  `ParseResult<T>`, whose failure is a `ParseFailure` (`src/input.rs`):
  `NoMatch(offset)` for a backtrack (this parser did not match here, an
  alternative may) or `Error(Box<Diagnostic>)` for a cut (the error that will
  be reported). Report a real error with `Err(cut(Diagnostic::expected(...)))`
  or another `Diagnostic` constructor. A backtrack is only a position, which
  is what makes trying alternatives free; `backtrack(offset)` makes one by
  hand. `parse_builtin_commands` adds the keyword as context to every error
  of a statement (`ParseFailure::with_context`).
* **Keyword statements use `KeywordCall`** (`src/parser/parse_keywords.rs`),
  which gives them nu's handling of the flags of a keyword command; see the
  recipe below.

## Recipe: add a binary operator

1. `src/ast/mod.rs`: add the variant to `Math`, `Comparison`, `Boolean` or
   `Bits`; give it a `precedence` (copy nu-protocol's table), an `as_str`, and
   a `from_spelling` arm (several spellings may map to one operator, like
   `=~` and `like`). If it is right-associative, add it to
   `is_right_associative`.
2. `src/parser/parse_expressions.rs`: `infix_operator` takes the binding
   power from `Operator::precedence` and the associativity from
   `Operator::is_right_associative`, so there is nothing to do unless the
   operator needs a "did you mean" hint for a common misspelling; add that to
   the `help` match of `parse_operator`.
3. `src/flatten.rs`: `Boolean` operators map to `FlatShape::Boolean`, others
   to `Operator`; check the match.
4. Tests: `all_operators_parse` in `tests/syntax.rs` iterates the spellings;
   add yours, and a precedence assertion if it is not obvious.

## Recipe: add a keyword statement

Suppose Nushell gains `unless COND { }`.

1. `src/ast/mod.rs`: add `pub struct Unless<'a> { pub condition:
   Box<Expression<'a>>, pub body: Block<'a> }` and `Expr::Unless(Unless<'a>)`,
   and add `Expr::Unless(_) => "unless"` to `Expr::keyword` so consumers can
   find the keyword's span (`Expression::keyword_span`).
2. The parser. Put the function in the file named after the nu-parser file
   that parses the keyword; a keyword that nu parses as an ordinary command
   (as it does `if` and `while`, and would `unless`) goes in
   `src/parser/parse_control_flow.rs`. Write `parse_unless` modelled on
   `parse_while`, and add `"unless" => ("unless", parse_unless(tokens))` to
   the match in `parse_builtin_commands` (`parse_expressions.rs`):

   ```rust,ignore
   /// `unless condition... { block }`: the condition is every item before
   /// the last, which is the block.
   pub fn parse_unless<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
       let mut call = KeywordCall::start(&mut tokens)?;
       call.flags(&mut tokens)?;
       let Some((block, condition)) = tokens.remaining().split_last().filter(|(_, condition)| !condition.is_empty())
       else {
           if call.wants_help() {
               return call.help_call();
           }
           return Err(cut(Diagnostic::expected("condition and block", tokens.end_span())));
       };
       let start = tokens.position();
       let condition = parse_math_expression(tokens.slice(start..start + condition.len()))?;
       let body = parse_block_argument(tokens.working_set, block, "block")?;
       let span = call.keyword.span.merge(block.span);
       call.finish(Expression::new(Expr::Unless(Unless { condition: Box::new(condition), body }), span))
   }
   ```

   Every keyword is a command with a fixed signature in nu, and
   `KeywordCall` reproduces what nu does with its flags. Start with
   `KeywordCall::start` (or `KeywordCall::start_positional` for a keyword nu
   parses by position, for which `--` is not an end-of-options marker). Take
   each positional item with `call.positional(&mut tokens, "what")`: it reads
   the flags at that boundary first (`--help`/`-h` is noted, the first `--` is
   consumed, any other flag is an error) and returns `None` for a missing
   positional that a `--help` forgives, in which case return
   `call.help_call()`. Where the shape is not one item per positional, read
   the flags with `call.flags(&mut tokens)` and take the rest with the stream
   methods or combinators (`tokens_until("else")`, `opt(keyword("else"))`,
   `alt((keyword("catch"), keyword("finally")))` wrapped in `expected`).
   Check the end with `call.end(&mut tokens)` and wrap the finished node in
   `call.finish(expression)`, which turns the statement into the ordinary
   call nu makes of `unless --help` when help was asked for.

   Then the keyword tables. `parse_builtin_commands` lets a `def` in the file
   shadow a keyword, as nu lets a definition shadow its keyword commands,
   unless it is a statement keyword. If the keyword must only appear at a
   pipeline head (like `def`), add it to `is_statement_keyword`
   (`parse_keywords.rs`); if it can be an operand (like `if`), add it to the
   keyword list of `is_math_expression_like` and to the `"if" | "match"`
   checks of `parse_math_expression` and `parse_math_operand`
   (`parse_expressions.rs`). If a `def` may not use the name, add it to
   `ALIASABLE_PARSER_KEYWORDS` or `UNALIASABLE_PARSER_KEYWORDS`, whichever
   nu's table of the same name has it in; `is_parser_keyword` reads both.
   Text that nu accepts and never looks at goes to
   `working_set.add_ignored(span)` rather than into the tree.
3. `src/ast/visit.rs`: descend into the condition and body in
   `walk_expression`.
4. `src/flatten.rs`: the keyword shape is pushed for you from
   `keyword_span()`; visit the condition and call `block_braces` for the body.
5. `src/pretty.rs`: print it.
6. `examples/nufmt/format.rs`: emit it (the formatter matches on `Expr`
   and has a wildcard fallback that copies the source text, so this can be
   done later, but the round-trip test will show the raw text until then).
7. Tests in `tests/syntax.rs`, fixtures under `tests/fixtures/accept/` and
   `tests/fixtures/reject/` with their golden files
   (`UPDATE_FIXTURES=1 cargo test --test fixtures`), and a line in
   `tests/corpus/kitchen_sink.nu` if nu accepts it.

## Recipe: add a literal form

1. `src/parser/parse_literals.rs`: a recogniser over `&str` returning
   `Option` (like `parse_filesize`), or a winnow parser over `&str` if it has
   structure worth expressing with combinators: `is_datetime` is
   `(date, opt((time, opt(offset))))` and `parse_int` an `alt` of the decimal
   form and the `0x`, `0o` and `0b` prefixes.
2. `parse_any_value` in `src/parser/parse_expressions.rs`: insert the
   attempt at the right place in nu's order (`null`, booleans, binary, range,
   filesize, duration, datetime, int, float, string). Also update
   `is_math_expression_like` if the literal can start a math expression, and
   `parse_value_for_shape` if a declared shape (a parameter default such as
   `[x: int = 1]`) should accept it.
3. Add an `Expr` variant if needed, then visitor, flatten, pretty.
4. Unit tests in the `tests` module of `parse_literals.rs`, behaviour tests
   in `tests/syntax.rs`, value tables in `tests/language.rs`.

## Recipe: change how a construct is lexed

Add a `LexOptions` preset rather than modifying the scanner. If the scanner
itself must change (a new bracket kind, a new quoting form), change
`item_length` and `interp_subexpr_step` in `src/lex.rs` together and check
nu-parser's `lex_item`, because the lexer and the interpolation parser
(`parse_interpolation_parts` in `parse_literals.rs`, which uses
`interp_subexpr_step` too) must agree on where a string ends.

## Debugging

```text
cargo run --example parse -- file.nu            # tree with spans
cargo run --example parse -- --flat file.nu     # what flatten sees
echo 'snippet' | cargo run --example parse      # from stdin
cargo run --release --example parse -- --check ~/src/nu_scripts   # find files that fail
nu -n -c 'ast --flatten "snippet"'              # what nu makes of it
```

A failing corpus file prints a rendered diagnostic with the line and a caret;
compare with `nu-check --debug file.nu`.

## Performance

The hot path is `parse_value` on bare words: every argument goes through the
literal attempts of `parse_any_value`, and `is_math_expression_like` runs on
every command head. Keep those attempts allocation-free on failure
(`parse_unit_value` checks the first bytes before uppercasing;
`find_longest_decl` only joins words when `is_decl_name_prefix` says the first
word starts a multi-word command). Backtracking is cheap because a backtrack
is a position and nothing else; keep it that way, and do not build a
`Diagnostic` for a failure that an `alt` or `opt` may throw away.

Measure a change with criterion's named baselines:

```text
cargo bench --bench parse -- --save-baseline before    # on the code before the change
cargo bench --bench parse -- --baseline before         # after it: the change of every benchmark
```

Rewriting a loop with combinators should not slow the `parse/`, `snippets/`
or `lexer/` groups down. A combinator can cost speed that the loop did not
(an extra pass, a `Vec` collected only to be walked again); if it does and
cannot be avoided, keep the loop. `parse --check` over `nu_scripts` gives the
whole-corpus throughput (see `how-to.md`).

## Pitfalls

* **Give a token stream its end.** `Tokens::new(working_set, tokens, end)`
  needs the byte offset after the last token so that "expected X" at the end
  of the input has a position (`tokens.here()`, `tokens.end_span()`). Use
  `Tokens::from_lexed(working_set, &lexed)` for the lexer's output (the `Eof`
  token gives the end), `tokens.slice(start..end)`, `tokens.rest_stream()` or
  `lite_command.tokens(working_set)` rather than building one by hand.
* **Spans are absolute.** When you slice an item to parse a part of it, pass
  the absolute start (`Span::new(span.start + k, ...)`, `lex(text, base, ..)`).
* **Cut, don't backtrack, for errors.** Return `cut(Diagnostic::...)` for a
  real error, and commit a combinator with `expected(what, parser)` or
  `cut_with(parser, error)` as soon as the input can only be this construct.
  A backtrack carries no message: one that escapes to the top is reported as
  a confusing "expected valid syntax" (`ParseFailure::into_diagnostic`). The
  other way round, a cut inside `alt`, `opt` or `repeat` stops the remaining
  branches, so cut only where no alternative could match.
* **Backtracking does not undo the working set.** A combinator that
  backtracks resets the stream's position, but whatever a parser recorded in
  the working set on the way (an error, a comment, ignored text, a
  predeclared name) stays. Record only after the parser has committed. The
  one place this is undone on purpose is `parse_help_call`, which calls
  `working_set.remove_ignored_from(offset)` before parsing a keyword
  statement again as a call.
* **Block recovery records errors.** A block that fails to parse still
  returns its pipelines with the errors recorded in the working set, so never
  parse a block to find out whether an item is one; decide from the text
  first (`brace_shape` in `parse_expressions.rs`, which classifies a `{...}`
  as `BraceShape::{Empty, ClosureParams, Record, Spread, Other}` the way nu's
  `parse_brace_expr` probes it, `is_range_syntax` and
  `is_math_expression_like` are the existing examples).
* **Comments are recorded where they are lexed.** If you re-lex an interior
  with `skip_comments: false`, call `working_set.add_comments(&tokens)` (or
  handle `Comment` tokens with `working_set.add_comment(span)`) so they are
  kept; if you lex the same text twice, the `dedup` in
  `WorkingSet::into_collected` at the end of the parse protects you, but avoid
  it.
* **Match nu's leniencies and strictnesses.** Do not "fix" `[1 | 2]`, `alias
  x = a | b`, `$x.a.` or `"a"b"c"`; they are accepted by nu. Do not accept
  `"abc"def`, `&&`, `def if [] {}`; nu rejects them.
* **`Expr` is `#[non_exhaustive]`.** Inside the crate `match` must be
  exhaustive; outside (examples, tools) a wildcard arm is required.
* **Keep `ast::visit`, `flatten`, `pretty` and the formatter in step** with
  any node change; clippy will not tell you about a missing descent.
