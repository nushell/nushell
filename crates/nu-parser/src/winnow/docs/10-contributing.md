# 10 Contributing: recipes and pitfalls

## Before you change the grammar

1. **Find the reference behaviour.** `nu -n -c '...'` on the local binary answers most questions in
   seconds, and `crates/nu-parser/src/*.rs` answers the rest. Write down what nu does, including the
   error cases, and which function does it.
2. **Find the file.** The nu-parser function you found usually has a namesake here, in the file of
   the same name. [Chapter 01](01-architecture.md#module-maps) has the full map.
3. **Add the test first** in `tests/winnow/syntax.rs`. It should fail.
4. **Implement, then check:** `cargo test -p nu-parser`, `cargo fmt --all`, `cargo nuclippy`,
   `typos`, the comparison scripts if the lexer or `parse_value` changed, and the benchmarks if you
   touched a hot path. Front-end changes also need `frontends compare` over the corpus.

## Conventions

* **Names follow nu-parser and nu-protocol.** A function that does the job of a nu-parser function
  has its name and lives in the file named like nu-parser's. Where nu has no counterpart, name it in
  the same style (`parse_while`, `parse_match_arm`).
* **Descriptive variable names.** `working_set`, `tokens`, `token`, `keyword`, `expression`,
  `operator_token`, not `st`, `c`, `tok`, `kw`, `e`.
* **Parsers of one item take the working set first**, then the item's span: `fn
  parse_record<'a>(working_set: &WorkingSet<'a>, span: Span) -> ParseResult<Expression<'a>>`. They
  read the text with `get_span_contents`, and a bracketed item is lexed again from its inside
  (`delimited_interior`, then `lex`).
* **Parsers of a sequence read a `Tokens` stream.** A statement parser takes the stream of its
  command's items, `fn parse_def<'a>(tokens: Tokens<'_, 'a>)`. The parts it is made of are winnow
  parsers, `fn(&mut Tokens<'_, 'a>) -> ParseResult<T>`, written with winnow's combinators and the
  token parsers of chapter 03.
* **A few parsers stay loops,** because they mirror a state machine of nu-parser or read more
  clearly that way: `parse_block`'s statement loop, `parse_lite_command`, `lite_parse_parts`,
  `parse_parameters`, `parse_interpolation_parts`, `parse_external_string`, `find_range_operators`
  and `item_length`.
* **Errors are cuts, and backtracks carry no diagnostic.** Report a real error with
  `Err(cut(Diagnostic::expected(...)))` or another `Diagnostic` constructor. `backtrack(offset)`
  makes a backtrack by hand.
* **An error nu goes past is recovered** (chapter 03). Record it with `as_recovered`, and leave a
  garbage leaf, the call form or the node so the lowering can report nu's error.
* **Keyword statements use `KeywordCall`** (chapter 05).
* **Lowering functions name their nu-parser counterpart** in their documentation, and read text only
  through the ports in `lower/`. Add a nu-parser function to `SHARED` in
  `tests/winnow/nu_parser_uses.rs` only if it reads no source text.

## Recipe: add a binary operator

1. `src/winnow/syntax/ast/mod.rs`: add the variant to `Math`, `Comparison`, `Boolean` or `Bits`,
   with a `precedence` from nu-protocol's table, an `as_str` and a `from_spelling` arm. Several
   spellings may map to one operator, like `=~` and `like`. If it is right-associative, add it to
   `is_right_associative`.
2. `parse_expressions.rs`: `infix_operator` takes the binding power and associativity from
   `Operator`, so there is nothing to do unless the operator needs a "did you mean" hint, which goes
   in the `help` match of `parse_operator`.
3. `flatten.rs`: `Boolean` operators map to `FlatShape::Boolean` and others to `Operator`. Check the
   match.
4. Tests: `all_operators_parse` in `tests/winnow/syntax.rs` iterates the spellings. Add yours, and a
   precedence assertion if it isn't obvious.
5. The lowering's `math_items.rs` and `expression.rs` map operators to nu-protocol's. Check them.

## Recipe: add a keyword statement

Suppose Nushell gains `unless COND { }`.

1. `ast/mod.rs`: add `pub struct Unless<'a> { pub condition: Box<Expression<'a>>, pub body:
   Block<'a> }` and `Expr::Unless(Unless<'a>)`, and add `Expr::Unless(_) => "unless"` to
   `Expr::keyword`.
2. The parser. A keyword nu parses as an ordinary command goes in `parse_control_flow.rs`. Write
   `parse_unless` modelled on `parse_while`, and add an `"unless"` arm to the match in
   `parse_builtin_commands`:

   ```rust,ignore
   /// `unless condition... { block }`: the condition is every item before
   /// the last, which is the block.
   pub fn parse_unless<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
       let mut call = KeywordCall::start(&mut tokens)?;
       if call.call_flags(&mut tokens)? {
           return call.help_call();
       }
       let Some((block, condition)) = tokens.remaining().split_last().filter(|(_, condition)| !condition.is_empty())
       else {
           call.missing();                    // nu's MissingPositional, the consumer's
           return call.help_call();
       };
       let start = tokens.position();
       let condition = parse_math_expression(tokens.slice(start..start + condition.len()))?;
       let body = parse_block_argument(tokens.working_set, block, "block")?;
       let span = call.keyword.span.merge(block.span);
       call.finish(Expression::new(Expr::Unless(Unless { condition: Box::new(condition), body }), span))
   }
   ```

   `parse_while` also wraps this in `call_after_error`, so an error nu's call goes past becomes the
   call form. Use `KeywordCall::start_positional` for a keyword nu parses by position, for which
   `--` doesn't end options. Take each positional with `call.positional(&mut tokens, "what")`. Where
   the shape isn't one item per positional, read flags with `call.flags` and take the rest with
   stream methods or combinators.

3. The keyword tables. `parse_builtin_commands` lets a `def` in the file shadow a keyword unless it
   is a statement keyword. If the keyword must only appear at a pipeline head, like `def`, add it to
   `is_statement_keyword`. If it can be an operand, like `if`, add it to the keyword list of
   `is_math_expression_like` and to the `"if" | "match"` checks of `parse_math_expression` and
   `parse_math_operand`. If a `def` may not use the name, add it to `ALIASABLE_PARSER_KEYWORDS` or
   `UNALIASABLE_PARSER_KEYWORDS`, whichever nu's table has it in. Text nu accepts and never looks at
   goes to `working_set.add_ignored(span)`.
4. `ast/visit.rs`: descend into the condition and body in `walk_expression`.
5. `flatten.rs`: the keyword shape comes from `keyword_span()`. Visit the condition and call
   `block_braces` for the body.
6. `pretty.rs`: print it.
7. `lower/keyword.rs`: lower it to what nu-parser builds for the same statement.
8. `examples/winnow_nufmt/format.rs`: emit it. The formatter has a wildcard fallback that copies
   source text, so this can wait, but the round-trip test shows the raw text until then.
9. Tests in `tests/winnow/syntax.rs`, fixtures with golden files, and a line in
   `tests/winnow/corpus/kitchen_sink.nu` if nu accepts it.

## Recipe: add a literal form

1. `parse_literals.rs`: a recogniser over `&str` returning `Option`, like `parse_filesize`, or a
   winnow parser if it has structure worth expressing with combinators, like `is_datetime`.
2. `parse_any_value`: insert the attempt at the right place in nu's order (`null`, booleans, binary,
   range, filesize, duration, datetime, int, float, string). Update `is_math_expression_like` if the
   literal can start a math expression, and `parse_value_for_shape` if a declared shape should
   accept it.
3. Add an `Expr` variant if needed, then visitor, flatten, pretty and the lowering's `value.rs` and
   `leaf.rs`.
4. Unit tests in `parse_literals.rs`, behaviour tests in `tests/winnow/syntax.rs`, value tables in
   `tests/winnow/language.rs`.

## Recipe: change how a construct is lexed

Add a `LexOptions` preset rather than changing the scanner. If the scanner itself must change (a new
bracket kind, a new quoting form), change `item_length` and `interp_subexpr_step` together and check
nu's `lex_item`, because the lexer and `parse_interpolation_parts` must agree on where a string
ends.

## Debugging

```text
cargo run --example winnow_parse -- file.nu            # tree with spans
cargo run --example winnow_parse -- --flat file.nu     # what flatten sees
cargo run --release --example winnow_parse -- --check ~/src/nu_scripts   # find files that fail
nu -n -c 'ast --flatten "snippet"'                     # what nu makes of it
```

Pipe a snippet into `winnow_parse` to read from stdin. A failing corpus file prints a rendered
diagnostic with the line and a caret, which you can compare with `nu-check --debug file.nu`.

For the front end, `frontends --log` and `nu --log-level debug --log-include nu_parser::winnow`
print each statement the front end couldn't read, with the reason. `frontends compare --show` prints
both renderings of a file.

## Performance

The hot path is `parse_value` on bare words: every argument goes through the literal attempts of
`parse_any_value`, and `is_math_expression_like` runs on every command head. Keep those attempts
allocation-free on failure. `parse_unit_value` checks the first bytes before uppercasing, and
`find_longest_decl` joins words only when `is_decl_name_prefix` says the first word starts a
multi-word command. Don't build a `Diagnostic` for a failure that an `alt` or `opt` may throw away.

Rewriting a loop with combinators shouldn't slow the `parse_*`, `snippets_*` or `lexer_*`
benchmarks. A combinator can cost speed the loop didn't, through an extra pass or a `Vec` collected
only to be walked again. If it does and can't be avoided, keep the loop.
[how-to.md](how-to.md#benchmarks) shows how to compare two builds.

## Pitfalls

* **Give a token stream its end.** `Tokens::new(working_set, tokens, end)` needs the byte offset
  after the last token, so "expected X" at the end of the input has a position. Prefer
  `Tokens::from_lexed`, `tokens.slice(start..end)`, `tokens.rest_stream()` or
  `lite_command.tokens(working_set)`.
* **Spans are absolute.** When you slice an item to parse a part of it, pass the absolute start
  (`Span::new(span.start + k, ...)`, `lex(text, base, ..)`).
* **Cut, don't backtrack, for errors.** A backtrack that escapes to the top is reported as a
  confusing "expected valid syntax". The other way round, a cut inside `alt`, `opt` or `repeat`
  stops the remaining branches, so cut only where no alternative could match.
* **Backtracking doesn't undo the working set.** A combinator that backtracks resets the stream's
  position, but what a parser recorded on the way stays. Record only after the parser has committed.
  The one deliberate undo is `parse_help_call`, which calls `remove_ignored_from`.
* **Block recovery records errors.** Never parse a block to find out whether an item is one. Decide
  from the text first, with `brace_shape`, `is_range_syntax` or `is_math_expression_like`.
* **Comments are recorded where they are lexed.** If you lex a region without `skip_comments`, call
  `working_set.add_comments(&tokens)` so they are kept. The dedup in `into_collected` protects a
  region lexed twice, but avoid it.
* **Match nu's leniencies and strictnesses.** Don't "fix" `[1 | 2]`, `alias x = a | b`, `$x.a.` or
  `"a"b"c"`, which nu accepts. Don't accept `"abc"def`, `&&` or `def if [] {}`, which nu rejects.
* **`Expr` is `#[non_exhaustive]`.** Inside nu-parser `match` must be exhaustive. Outside, in
  examples and tools, a wildcard arm is required.
* **Keep `ast::visit`, `flatten`, `pretty` and the formatter in step** with any node change. Clippy
  won't tell you about a missing descent.
* **New fixtures need a touch.** `rstest`'s `#[files]` expands at compile time, so touch
  `tests/winnow/fixtures.rs` for cargo to see a new file.
