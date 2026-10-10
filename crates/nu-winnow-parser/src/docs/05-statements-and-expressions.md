# 05 Statements and expressions

Files: `src/parser/parse_expressions.rs` (dispatch, environment shorthand,
assignments, math), `src/parser/parse_keywords.rs` (what all keyword
statements share), `src/parser/parse_calls.rs` (calls), and one file per
group of statements: `parse_def.rs`, `parse_bindings.rs`, `parse_alias.rs`,
`parse_module.rs`, `parse_source.rs` and `parse_control_flow.rs`.

The input to this layer is a `Tokens` stream over the items of one command
(`LiteCommand::tokens`, chapter 04) and its `Position`. The output is one
`Expression`. Parsers of a sequence of items take the stream and compose with
winnow combinators; parsers of one item take `(working_set, span)`, like
nu's `parse_value(working_set, span, shape)`.

## Dispatch order (`parse_expression`)

```rust,ignore
pub enum Position { Statement, Element }

pub fn parse_expression<'a>(mut tokens: Tokens<'_, 'a>, position: Position) -> ParseResult<Expression<'a>> {
    // 1. In statement position, statement keywords parse their own `=` and `{}` arguments.
    if position == Position::Statement && is_statement_keyword(first_text) { return parse_builtin_commands(tokens); }
    // 2. `NAME=value` prefixes: `repeat(0.., env_assignment)`.
    let env_assignments = parse_env_shorthand_prefix(&mut tokens)?;
    // 3. After shorthand, or as a pipeline element, the head is checked against nu's table.
    if !env_assignments.is_empty() || position == Position::Element { check_builtin_command_in_pipeline(&tokens)?; }
    // 4. Any assignment operator among the items -> assignment.
    let expression = if tokens.remaining().iter().any(|token| matches!(token.contents, TokenContents::AssignmentOperator(_))) {
        parse_assignment_expression(tokens.rest_stream())?
    // 5. A first item that looks like a value -> math expression.
    } else if is_math_expression_like(first_text) {
        parse_math_expression(tokens.rest_stream())?
    // 6. Otherwise a keyword expression (if, match, ...) or a call.
    } else {
        parse_builtin_commands(tokens.rest_stream())?
    };
    /* wrap `expression` in EnvShorthand if `env_assignments` is not empty */
}
```

This is nu-parser's `parse_expression` in the same order, with nu's
distinction between a command that is a whole pipeline (nu's
`parse_builtin_commands`, `Position::Statement`) and one that is an element
of a longer pipeline, the left side of an assignment, an `else` or a match
arm (nu's `parse_expression`, `Position::Element`). `parse_pipeline`
collects every command before parsing any (chapter 04) so that it knows which
case applies. Details that are easy to miss:

* `is_math_expression_like` is nu's function of the same name: true for
  `true`, `false`, `null`, `not`, `if`, `match`, items starting with
  `( { [ $ " ' -` or `r#`, and items whose text is a number, unit, datetime,
  binary or range whose bounds parse (a `$` bound is a variable name and a
  valid cell path, so `..$x.c!!` is an external command). It is a pure function of the text, so `-1 | math abs` is
  math, `"ls"` is a string not a call, and `1 + 1` at a pipeline head is
  arithmetic, all without parsing anything twice.
* Statement keywords (`is_statement_keyword`: `def`, `let`, `mut`, `const`,
  `for`, `alias`, `module`, `use`, `export`, `export-env`, `extern`) are
  checked *before* the assignment test because `let x = 1` contains an
  `AssignmentOperator` token.
* `check_builtin_command_in_pipeline` is nu's head table for an element: the
  declaration keywords, `for`, `module`, `use`, `source`, `hide`, `export`,
  `export-env` (nu's `BuiltinCommandInPipeline`), `const` and `mut` (nu's
  `AssignInPipeline`), `overlay` unless the second item of the command is
  `list`, and `plugin` when the second item is `use` are errors there. So
  `FOO=1 def x [] {}` and `hide ls | length` are refused, while
  `ls | overlay list` parses. The second item is counted from the start of
  the command, shorthand included, as nu counts its spans. `let` is not in
  the table: `ls | let x` parses (nu accepts it too), and `ls | let x = 1` is
  refused as an assignment whose left side is not a variable, because after
  a `|` the keyword is not a statement.

## Keyword forms (`parse_builtin_commands`)

`parse_builtin_commands` matches the head text and calls one function per
keyword; anything else is a call (`parse_call`). A non-statement keyword
(`if`, `loop`, `where`, ...) that a user command shadows
(`working_set.is_declared`) is a call. So is `if`, `match`, `while`, `loop`,
`try`, `return`, `break` or `continue` when a longer known command name
starts with it: `def "if ready" [] {}` makes `if ready` a call. nu parses
these keywords with `parse_call`, and its longest match
(`find_longest_decl`, below) finds the longer name. `where` and the
statement keywords go by their first word alone. The match also attaches
the grammar context (`while parsing for`) to any error. The keyword
functions live in files named like nu-parser's:

| File | Functions |
| --- | --- |
| `parse_def.rs` | `parse_def`, `parse_extern`, `parse_for`, `parse_def_predecl` (chapter 04) |
| `parse_bindings.rs` | `parse_let`, `parse_mut`, `parse_const` (sharing a private `parse_binding`) |
| `parse_alias.rs` | `parse_alias` |
| `parse_module.rs` | `parse_module`, `parse_use`, `parse_export_in_block`, `parse_export_env` |
| `parse_source.rs` | `parse_where` (nu keeps it in this file) |
| `parse_control_flow.rs` | `parse_if`, `parse_match`, `parse_while`, `parse_loop`, `parse_try`, `parse_return`, `parse_break_or_continue`; nu-parser has no such file, because these are ordinary commands there |
| `parse_keywords.rs` | what they share: `is_statement_keyword`, `ALIASABLE_PARSER_KEYWORDS`, `UNALIASABLE_PARSER_KEYWORDS`, `KeywordCall`, `keyword_boundary`, `parse_help_call`, `parse_block_argument` |

Each keyword function takes the command's stream and follows the same
pattern: `KeywordCall::start` consumes the keyword, `call.positional` takes
each positional item (after looking for flags, next section), items go to
`parse_value` with the right `ExpectedShape` (or to `parse_block_argument`
for a block), and `call.end` and `call.finish` close the statement:

```rust,ignore
pub fn parse_for<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;                          // consumes `for`
    let Some(variable) = call.positional(&mut tokens, "loop variable")? else { return call.help_call() };
    let (var, typed) = parse_var_with_opt_type(working_set, &variable)?;      // `x`, `$x` or `x:` (typed)
    let ty = match typed {
        true => /* `tokens_until("in")`: every item before `in`, parsed as one type */,
        false => None,
    };
    let Some(in_keyword) = call.positional(&mut tokens, "`in`")? else { return call.help_call() };
    if tokens.text(&in_keyword) != "in" {
        return Err(cut(Diagnostic::new(ErrorKind::ExpectedKeyword("in"), in_keyword.span)));
    }
    if tokens.remaining().len() < 2 { return Err(/* missing argument to `in`: the last item is the block's */); }
    let iterable = tokens.expect_item("value to iterate")?;                  // never forgiven
    let iterable = Box::new(parse_value(working_set, iterable.span, ExpectedShape::Any)?);
    let Some(block) = call.positional(&mut tokens, "block")? else { return call.help_call() };
    let body = parse_block_argument(working_set, &block, "block")?;           // must start with `{`
    call.end(&mut tokens)?;                                                   // trailing flags, then the end
    let span = call.keyword.span.merge(block.span);
    let for_loop = For { var, ty, in_keyword: in_keyword.span, iterable, body };
    call.finish(Expression::new(Expr::For(Box::new(for_loop)), span))
}
```

Fixed shapes are written with the token parsers of `tokens.rs` (`item`,
`keyword(word)`, `expected`, `tokens_until`) and winnow's combinators. `if`
splits its items at the `else` keyword:

```rust,ignore
let then_part = tokens_until("else").parse_next(&mut tokens)?;     // a stream over the items before `else`
let else_keyword = opt(keyword("else")).parse_next(&mut tokens)?;   // Some(token) if `else` follows
```

`try` reads each handler keyword with
``expected("`catch` or `finally`", alt((keyword("catch"), keyword("finally"))))``,
and `def` reads its flags with `repeat(0.., def_flag)`, where `def_flag` is
`item.verify(..)` accepting an item that starts with `--` and is neither
`--help` nor `--`.

### Positional boundaries: `--help`, `--` and `-x`

Every keyword is a command with a fixed signature in nu, so at the start of
each of its positional arguments `parse_internal_call` looks for flags.
`KeywordCall` (nu's `parse_keyword` behaviour) reproduces that with its
methods `start`, `start_positional`, `flags`, `positional`, `end`,
`wants_help`, `help_call` and `finish`, built on `keyword_boundary`,
`keyword_boundary_with` and `seen_end_of_options`:

* `--help` or `-h` at a boundary makes the whole statement an ordinary
  call showing help (`help_call` calls `parse_help_call`, which drops the
  text the statement recorded as ignored and re-parses its items with
  `parse_call`, so `def foo [] --help` is a `Call` to `def` with three
  arguments). nu keeps parsing the positionals that
  follow the flag and only forgives the *missing* ones, so `call.flags`
  records the flag (`wants_help`) and parsing goes on: `match 1 --help :{}`
  still has no match block, `return --help 1 2` and `loop --help {} y` have
  an extra positional, `def foo [] {} {} --help` has no colon, but
  `match 1 --help` and `def foo --help` are fine. `call.positional`
  returning `None` is the "missing positional is forgiven" step; `finish`
  turns the finished statement into the help call. The argument of the `in`
  keyword is never forgiven: nu reserves the last item for the block before
  it reads the keyword's argument, so `for x in []` and `for x --help in []`
  are both "missing argument to `in`". A typed loop variable takes every
  item before `in` as its type (`for x: record<a: int, b: string> in [] {}`
  parses; `for x: int --help in [] {}` has the unknown type `int --help`).
* The first `--` at a boundary is consumed, recorded in `Ast::ignored` and
  otherwise dropped (`return -- 1` returns `1`). After it nu looks for no
  flags at all (`seen_end_of_options`): a second `--` is a positional
  (`try {} -- catch {} --` has one too many) and `return -- --help` returns
  the string `--help`.
* Any other `-x` at a boundary is "the `for` command doesn't have flag
  `-x`": `return -1`, `match -1 {}`, `if -1 > 0 {}` and `where -1 > 0` are
  errors, as they are in nu.
* Where a positional spans several items (a condition, a signature, an alias
  target) only its first item is a boundary.
* The statements nu parses by position rather than through
  `parse_internal_call` never see `--`: `export-env` starts its
  `KeywordCall` with `start_positional`, and `let`/`mut`/`const`, `alias` and
  `module` call `keyword_boundary_with(.., end_of_options = false)` and
  `parse_help_call` directly. So `alias -- x = ls`, `alias x -- = ls`,
  `module x -- {}`, `let -- x = 1` and `export-env -- {}` are errors. An
  `alias` help call must be the whole statement (`alias --help`,
  `alias x --help`); `alias --help x = ls` and `alias x --help extra` are
  "missing sign" (expected `=`). `export --help x` has an extra positional.
  `use` looks for its flags before every argument and consumes any number of
  `--`.

The table below is the contract each keyword implements. "items" are
whitespace-delimited items; "rest" means everything to the end of the line
(because of `=` absorption, this can contain pipes).

| Keyword | Shape | Notes |
| --- | --- | --- |
| `def` | `def [--env] [--wrapped] NAME SIG[:] [TYPES] BODY` | `NAME` must be a string item (`parse_def_name`: `def $x` is "expected string", a name containing `[` or `(`, even quoted, is "no space between name and parameters"); parser keywords, names with `#`/`^`/`%` and names that read as numbers are rejected (`check_definition_name`). The flags (`parse_def_flags`) may also follow the name. The signature argument takes every remaining item but the last (`parse_full_signature`): one item is the signature, two of which the second starts with `{` is the signature plus an item nu drops (`def foo [] {} {}`; the dropped one is `Ast::ignored`), otherwise `TYPES` follow a `:` attached to the signature or standing alone (`def foo [] : {}` is fine) and are re-lexed together in signature mode. `BODY` is parsed as a closure without a probe (`parse_def_body`): `{\|x\| }` keeps its parameters as `Def::body_params`, `{a: 1}` is a call to `a:`. `--wrapped` needs a rest parameter that is untyped or `string` (`check_wrapped_signature`). A bare name starting with `-` or a spread (`def - [] {}`, `def ...{x: 1} [] {}`) is refused, as nu reads neither as the name; a name is refused when Rust reads it as a float (`1e3`, `inf`) or the `bytesize` crate as a size (`1k`, `"1 kb"`), not by Nushell's own number syntax (`def 0x10` is fine). The help call of `def foo [] {}{} --help` leaves out the dropped item, as nu never parses it. |
| `extern` | `extern NAME SIG[:] [TYPES]` | The signature argument takes *every* remaining item, so a body (`extern foo [] {}`) is the dropped second item, never parsed; default values in an extern signature are not parsed either (chapter 07). |
| `let`/`mut`/`const` | `KW NAME[:] [TYPE...] [= rest]` | `NAME` may start with `$` (`parse_var_with_opt_type`); reserved names (`in`, `nu`, `env`, `ans`) are refused (`ensure_not_reserved_variable_name`); the type tokens are re-lexed together (`parse_type_after_var`); the value is the rest, parsed as a *block* (a pipeline). Only `let x` may leave the `=` out; `mut x` and `const x` are "missing required positional argument". A bare `$` is a name (`let $ = 1`, nu's `is_variable`). Without an `=`, nu parses `let` as an ordinary call, so a `--` before the name ends the options (`let -- x`); with one it is a name and refused. A type of several items is lexed as a signature first (`parse_var_type`), so `record<a: int, b]>` is unbalanced. |
| `for` | `for VAR[:] [TYPE] in ITEM BODY` | |
| `alias` | `alias NAME = rest` | The `=` must be the item right after the name (`alias x=y`, `alias x`, `alias x =`, `alias = x` are errors; `export alias x =` is accepted, a nu quirk whose length check counts the `export` word, and gives `Alias::value == None`). A bare name starting with `-` is "alias name not supported". The rest is handed to `parse_call_lenient(.., true)` as plain words (a pipe becomes a word, `alias x = FOO=1 ls` calls the external `FOO=1`), after two checks nu makes first: a target that `is_math_expression_like` (a literal, `$x`, `(..)`) is "cannot create an alias to an expression", and a target naming an unaliasable keyword (`UNALIASABLE_PARSER_KEYWORDS`: `def`, `let`, `for`, `export def`, ...) is "cannot create an alias to a parser keyword"; `if`, `match`, `try` and `overlay *` may be aliased. Lenient means a keyword-command target may miss positionals and flag values (`alias x = overlay new`). nu checks the items by position first (`check_alias_name`): with three or more after `alias` the second must be `=`, with one or two the first must not be. Any `--help` or short-flag batch with `h` before the `=` is then a help call (`alias --help = ls`, `alias -hx`). A lone `-` is a name; `$x`, `(x)`, `{x}` are not. `export alias h home` (one item, no `=`) defines nothing and is accepted, but not in a module body, where nu parses the alias without its `export`. An aliased `if`, `match` or `try` is parsed with the keyword's own rules, forgiving only missing positionals (`alias i = if x` is fine, `alias i = if x y` is not). An alias is declared once its statement is parsed; a call to an alias of an external command is an external call. |
| `module` | `module NAME [BODY]` | `NAME` is a literal string (`parse_module_name`): a record is a type error, `$x`, `(..)` and `$"..."` are "not a string". A body holds declarations only. One `--` after the body is dropped (`module x {} --`); in the body, an `export alias` must define an alias. A `--help` or `-h` anywhere before a `--` makes a help call, as nu decides it first: only the name is parsed (a string, so a record is refused), the item in the body's place is ignored text (`module x {ls} --help`), and a third item is one too many (`parse_module_help_call`). |
| `use` | `use MODULE [MEMBER...]` | `MODULE` is a string or `null` (a no-op after which the members are parsed and then ignored). Members (`parse_import_pattern_member`) are names, `*`, or a `[a b]` list (`parse_import_pattern_list`); only the last may be `*` or a list. A member that is a variable, subexpression or `key: value` record is parsed and kept as `ImportPatternMemberKind::Ignored`, as nu ignores it; non-string list items and a cell path after a list (`[math].x`) are `Ast::ignored`; anything else is "wrong import pattern structure". |
| `export` | `export def\|extern\|alias\|use\|module\|const ...` | Wraps the inner statement (`parse_export_in_block`). |
| `export-env` | `export-env BODY [ITEM...]` | nu hands `export-env` exactly one argument: extra items (`export-env {} extra`) and a redirection on it are `Ast::ignored`. A closure or record body is refused. |
| `if` | `if COND... BLOCK [else BLOCK\|EXPR]` | The condition is every item before the block; the block is the item before `else` or the last item. A `key:` record where the block should be is "expected block" when nu would type it `record<...>` (every key a plain string); nu types any other record, and a cell path on one, `any`, which it accepts as a block (`if true { $env.A:b }`, `if true {a: 1}.a`), and so does `parse_brace_value`, keeping it as `If::then_value`. The else branch is a block or a whole expression (`parse_block_or_value`, or `parse_expression` in `Position::Element`, so `else if ...` recurses and `else {\|x\| }` is a closure). The condition takes every item but the last; when that last item is `--help` the statement is a help call once the condition parses (`if x --help`, but not `if if --help`). A `$` or `(...)` item where the block belongs is kept as `If::then_value`, as nu type-checks it (`if $c $env.f`); the same holds for the bodies of `while`, `loop`, `try` and `for` (`body_value`). |
| `match` | `match ITEM BLOCK` | See chapter 07 for the block. nu decides what the `{ ... }` is before it knows it wants arms, so a `{\|x\| }` closure, a `{a: 1}` record, a `$x` or a `(..)` in that position is accepted and kept as `Match::value_block` (the arms are empty); `match 1 [a]` and `match 1 foo` are errors. |
| `while` | `while COND... BLOCK` | As `if`: the condition takes every item but the last, a final `--help` is a help call once the condition parses, and a `$`/`(...)` body is `While::body_value`. |
| `loop` | `loop BLOCK` | |
| `try` | `try BLOCK [catch\|finally CLOSURE]{0,2}` | Handlers are kept in source order; nu allows two of either kind. A handler is a closure, or a `$x`/`(..)` that may hold one (`parse_try_handler`); a record is refused unless nu would type it `any`, as for the block of `if` (`catch { $env.A:b }`, `finally {a: 1}.a`). |
| `return` | `return [ITEM]` | One item only, as in nu. |
| `break`/`continue` | keyword alone | |
| `where` | `where {closure}` or `where COND...` | A row condition: `parse_row_condition`. `where --help` is a help call. One `{` item is a closure when it ends with `}`; nu tries a closure first and falls back to a row condition, so `where {}.a` is a cell path on a record. |

How the extent of a multi-item argument is found deserves a note. nu-parser
computes it from the signature ("the condition of `if` gets all spans up to
the ones needed by the remaining required positionals"). Here that rule is
written out per keyword: the `if` condition is the stream `tokens_until("else")`
returns minus its last item (the block), `while` splits the remaining items
at the last one, and the `def` signature is every remaining item but the last
(a slice pattern on `tokens.remaining()`). This is the same result without
needing signatures.

### Redirections and attributes on statements

`parse_pipeline_element` (chapter 04) builds the element's
`PipelineRedirection` with `parse_redirection` and refuses one on the
statements nu refuses it on (`rejects_redirection`): `def`, `extern`, `let`,
`mut`, `const`, `for`, `alias`, `module`, `use`, `export`, an attribute
block, any call whose head word is `overlay` (refused by name before the
arguments are looked at) and the calls whose keyword signature (below) is not
`redirectable` (`hide`, `source`, `run`, `plugin use`). A redirection on
`export-env` is dropped without a look, as nu drops it, and recorded in
`Ast::ignored`.

Attribute lines (`@name args`, collected by chapter 04) are parsed by
`parse_attribute` (`parse_calls.rs`) as calls to `attr name`: the name must be
non-empty, whether `attr name` exists is the consumer's business (it may come
from a `use`d module), and the built-in attributes (`category`, `complete`,
`deprecated`, `example`, `interactive`, `search-terms`) get their arguments
checked against their keyword signatures. Attributes may only precede `def`,
`extern`, `export def` and `export extern`; before anything else
(`export alias`, a call) they are "attributes must be followed by a
definition".

The keyword itself is not stored in the tree: `Expr::keyword()` names it
and `Expression::keyword_span()` locates it, since it is always the first
word of the expression's span (chapter 08).

## Math expressions (`parse_math_expression`)

Operands are single items parsed with `parse_value(.., ExpectedShape::Any)`;
operators are items whose text is an operator spelling. nu-parser folds them
with an operator-precedence stack. Here winnow's `expression` combinator, a
Pratt parser, does the precedence climbing, with nu's binding powers and
associativity:

```rust,ignore
pub fn parse_math_expression<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    /* an `if` or `match` at the start is the whole expression */
    expression(parse_math_operand).infix(infix_operator).parse_next(&mut tokens)
}

fn infix_operator<'t, 'a>(tokens: &mut Tokens<'t, 'a>)
    -> ParseResult<Infix<Tokens<'t, 'a>, Expression<'a>, ErrMode<ParseFailure>>>
{
    let token = expected("operator", item).parse_next(tokens)?;
    let operator = parse_operator(tokens.working_set, &token)?.item;
    let power = i64::from(operator.precedence());
    Ok(match operator.is_right_associative() {
        true => Infix::Right(power, fold_binary_op),
        false => Infix::Left(power, fold_binary_op),
    })
}
```

`fold_binary_op` builds the `BinaryOp`. winnow hands the fold only the two
operands, so it finds the operator as the token right after the left operand
(`tokens.token_after(lhs.span.end)`); `parse_operator` has already checked
its spelling.

Precedence lives on `ast::Operator::precedence` (100 for `**` down to 40 for
`or`, nu-protocol's values); `**` is the only right-associative operator.
`parse_math_operand` reads `not` prefixes with `repeat(0.., keyword("not"))`
and then one value. `if` and `match` are allowed as operands and consume the
rest of the items (`1 + if $x { 2 } else { 3 }`). Unknown operators get
targeted help from `parse_operator` (`^` → use `**`, `%` → `mod`,
`contains` → `has`, ...).

```rust
use nu_winnow_parser::{parse, ast::{BinaryOp, Expr, Operator, Math}};

fn binary<'e, 'a>(expr: &'e Expr<'a>) -> &'e BinaryOp<'a> {
    match expr {
        Expr::BinaryOp(b) => b,
        other => panic!("{other:?}"),
    }
}

let ast = parse("1 + 2 * 3 ** 2 ** 1").unwrap();
let top = binary(&ast.block.pipelines[0].elements[0].expr.expr);   // 1 + (2 * (3 ** (2 ** 1)))
assert_eq!(top.op.item, Operator::Math(Math::Add));
let product = binary(&top.rhs.expr);
assert_eq!(product.op.item, Operator::Math(Math::Multiply));
let power = binary(&product.rhs.expr);                              // `**` is right-associative
assert!(matches!(power.rhs.expr, Expr::BinaryOp(_)));
```

### Row conditions

`where` parses its condition with `parse_row_condition`: the math expression
is parsed as above, then `expand_row_condition` rewrites the finished tree in
place. Any *string* operand on the left of an operator, or a lone operand,
becomes a cell path on the implicit `$it` (`expand_to_cell_path`, nu-parser's
name): `where size > 1kb` yields `FullCellPath { head: Var it (empty span),
implicit_head: true, tail: [size] }`. Only left operands are expanded, as in
nu-parser.

```rust
use nu_winnow_parser::{parse, ast::Expr};

let ast = parse("ls | where size > 1kb").unwrap();
let Expr::Where(where_) = &ast.block.pipelines[0].elements[1].expr.expr else { panic!() };
let Expr::BinaryOp(condition) = &where_.condition.expr else { panic!() };
let Expr::FullCellPath(path) = &condition.lhs.expr else { panic!() };
assert!(path.implicit_head);
assert_eq!(path.tail.len(), 1);
assert!(matches!(condition.rhs.expr, Expr::Filesize(_)));   // the right operand stays a value
```

## Assignments

`parse_assignment_expression` splits the items at the first
`AssignmentOperator` token. The left side is parsed with `parse_expression`
in `Position::Element` and must be what nu's `parse_full_cell_path`
produces: a `Var`, a `Subexpression`, a `FullCellPath`, a `List`, a `Table`
or a `key: value` `Record`, with or without a cell path (nu accepts
`(1) = 2` and `[1].0 = 2` at parse time and fails at run time); anything else
is "assignment requires a variable". The right side (everything absorbed to
the end of the line) is parsed with `parse_block` and stored as a `Block`,
matching nu, where `$x = ls | length` assigns the pipeline's result. Since
nu 0.97 an external command at the start of the value must be written with
a caret: an `ExternalCall` without one as the first element of the rhs is
"external command calls must be explicit in assignments" (`$x = git` is
refused, `$x = ^git` is not). A bare external head needs the command table
(or an alias of an external command), so with no table this never fires.

## Calls (`parse_call`, `find_longest_decl`, `parse_call_arguments`)

`find_longest_decl` implements nu's longest-match rule: try the first *n*
words (up to five with a `ParseConfig` table, any number with a
`CommandLookup`) joined with spaces against the known commands
(`working_set.find_decl`), longest first; `str trim --left` becomes a call to
`str trim` with one flag. It also returns the `DeclKind` the lookup gave for
a name of several words. For a single-word head it returns `None`, since it
never looked that word up, and the caller asks `find_decl` itself.
Two performance details: the single-word fast path never builds a string,
and a multi-word attempt only happens if the first word is a known *prefix*
of some multi-word command (`working_set.is_decl_name_prefix`; `ParseConfig`
and the declaration scopes both index prefixes). The search itself is
`find_longest_name`, which takes the question to ask of each candidate name:
`find_longest_decl` asks `find_decl`, and the `%` sigil (below) asks
`is_builtin_decl`.

`parse_call_arguments` is `repeat_to_end(parse_call_argument)`:

```rust,ignore
fn parse_call_arguments<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Vec<Argument<'a>>> {
    repeat_to_end(parse_call_argument).parse_next(&mut tokens)
}
```

`repeat_to_end(p)` is winnow's `repeat_till(0.., p, eof)`, so every item must
be an argument; `parse_call_argument` reads one with
`expected("argument", item)` and classifies it with one `match`:

| Item | Result |
| --- | --- |
| `--` | `Argument::EndOfOptions` |
| `--name`, `--name=value` | `Argument::Named(NamedArgument { long: true, value, .. })` (value only for the `=` form) |
| `-x`, `-abc` | `Argument::Named(NamedArgument { long: false, name: "abc", .. })` unless it looks like a negative number (`-1`, `-.5`) |
| `...$x`, `...[..]`, `...(..)`, `...{..}` | `Argument::Spread` |
| anything else | `Argument::Positional(parse_value(.., ExpectedShape::Any))` |

Whether `--name value` binds `value` to the flag is a signature question and
is left to the consumer; the bridge in `tools/` shows how (chapter 09). The
one exception is the commands that are keywords in nu-parser and have a
fixed signature there: `hide`, `source`, `source-env`, `run`, `overlay new`,
`overlay use`, `overlay hide`, `overlay list`, `plugin use` and the
built-in `attr *` attributes. `parse_call_lenient(tokens, lenient)` (which
`parse_call` calls with `lenient = false`) finishes by calling `check_call`,
nu's name for the check. It looks the head up in the `keyword_signature`
table, whose `KeywordSignature` entries have named fields and start from
`KeywordSignature::NONE`:

```rust,ignore
"overlay use" => KeywordSignature {
    required: 1,
    keyword: Some("as"),
    flags: const { &[switch("prefix", Some('p')), switch("reload", Some('r'))] },
    accepts_nothing: true,
    ..NONE
},
```

The fields are the required and optional positional counts (`required`,
`optional`), a rest (`rest`), a keyword argument such as `overlay use`'s `as`
(`keyword`), the flags (`KeywordFlag { long, short, takes_value }`), whether
unknown flags pass (`allows_unknown_flags`, for `run`), whether `null` is
allowed as the first positional (`accepts_nothing`) and whether the call may
be redirected (`redirectable`). `check_call_arguments` walks the arguments
against it and reproduces nu's checks: unknown flags, a flag without its
value, too many positionals, `as` without its name, `-1` as a flag rather
than a number, and `null` or `true` where a string is required
(`hide null`). With no command table the head of `overlay use x` resolves as
`overlay` with `use` as its first argument; `keyword_signature_of_call`
finds both spellings. `lenient` is set for an alias target, where nu
forgives missing positionals and flag values. Whether the named module, file
or plugin exists is still the consumer's.

The `%` sigil (`parse_percent_call`) forces the built-in command even when a
custom command or alias shadows its name. `%ls` and `% ls` give a `Call`
whose `sigil` is the span of the `%`; `%$cmd` and `%(expr)` give a
`DynamicCall` whose head is the `$` expression or subexpression. Like nu
(`find_longest_decl_with_command_type`), the head is the longest name of a
built-in command, visible or not (`working_set.is_builtin_decl`), so
`%ls foo` calls `ls` with the argument `foo` even where `def "ls foo"`
exists. A quoted or otherwise non-bare name after `%` is an error, and a
bare name that is not a built-in command (`CommandType::Builtin`, so not a
keyword such as `if` nor a prelude command such as `pwd`) is rejected with
"percent sigil requires a built-in command" (skipped when the parse knows
no commands at all). The error points at the first word: under
`def "my cmd"`, `%my cmd` is reported at `my`.

## External calls and environment shorthand

`^cmd args` (`parse_external_call`): the head after `^` is a string,
`$var` or `(subexpr)`. The arguments are
`repeat_to_end(parse_external_call_argument)`: `...$x`, `...[..]` and
`...(..)` are spreads, `$..`/`(..)`/`[..]`/`{..}` are parsed as values, and
everything else is an **external string**. A `[..]` argument must end with
`]` (`check_external_list_argument`): nu hands it to its list parser alone,
so `^cmd [a].x` and `^cmd ...[a].x` are "unclosed delimiter" while
`^cmd (ls).name` parses. A bare head that resolves to no command is an
external command for nu too: with a command table configured,
`parse_call_lenient` hands it to `parse_external_arguments` like `^cmd`
(with `ExternalCall::caret` `None`), so `git log 0b2d1f4..HEAD` has three
external strings and `cmd [a].0` is unclosed. A call to an alias of an
external command (`alias g = git`) is an external call the same way
(`DeclKind::ExternalAlias`). A `def --wrapped` whose rest parameter has no
type gets nu's `external_arg` shape for it, so a call to it (or to an alias
of it) parses every argument as an external command's (`f 'x'$` and `f 0b2`
are strings; `DeclKind::Wrapped`, recorded by `parse_def_predecl`, whose
`has_untyped_rest` takes the rest parameter from the lexed signature and then
looks for its type on the text, as nu's `rest_param_is_type_annotated` does).
The call records it (`Call::wrapped`, chapter 08). nu gives any positional
before the rest its own shape, which is left to consumers here. `parse_external_string` reproduces
nu-parser's segmenting: a word is split into bare, quoted, backtick and
parenthesised segments (the `ExternalStringSegment` state machine, kept as a
byte loop), quoted segments stay literal, parenthesised bare segments
interpolate, and all-literal words collapse to one string
(`--query='q($x)'` keeps its parentheses; `--out=(pwd)/x` interpolates).

`FOO=bar BAZ=$x cmd`: leading items of the form `NAME=value` with an
identifier `NAME` (`is_env_variable_name`) become `EnvShorthand`; the value
is a `$` expression or a strict string. `parse_env_shorthand_prefix` is a
`repeat` over a parser of one such item:

```rust,ignore
fn parse_env_shorthand_prefix<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<Vec<EnvAssignment<'a>>> {
    repeat(0.., env_assignment).parse_next(tokens)
}

fn env_assignment<'a>(tokens: &mut Tokens<'_, 'a>) -> ParseResult<EnvAssignment<'a>> {
    let working_set = tokens.working_set;
    let (token, equals) = item
        .verify_map(|token| {
            let text = working_set.get_span_contents(token.span);
            let equals = text.find('=').filter(|equals| is_env_variable_name(&text[..*equals]))?;
            Some((token, equals))
        })
        .parse_next(tokens)?;
    /* the value after `=`: empty, a `$` expression, or a strict string */
}
```

The first item that is not `NAME=value` makes `verify_map` backtrack, which
ends the `repeat` and leaves the stream at the command's head. A line
consisting only of shorthand is an error ("unknown command").

## Stream idioms you will reuse

```rust,ignore
let keyword = tokens.expect_item("for")?;               // next Item, or "expected for" at the right span
if tokens.at_end() { .. }                               // nothing left
tokens.expect_end()?;                                   // ExtraTokens error otherwise
let rest: Option<Span> = tokens.consume_rest();         // consume the rest, get its span
let condition = tokens.slice(start..end);               // a sub-stream over some of the items
let items = tokens.all();                               // the underlying slice, for index arithmetic
let value = parse_value(tokens.working_set, token.span, ExpectedShape::Any)?;  // one item

// token parsers, composed with winnow combinators
let else_keyword = opt(keyword("else")).parse_next(&mut tokens)?;           // an item spelled `else`, or None
let before_in = tokens_until("in").parse_next(&mut tokens)?;                // a stream over the items before `in`
let arguments = repeat_to_end(parse_call_argument).parse_next(&mut tokens)?; // every item, or an error
let arrow = expected("`=>`", keyword("=>")).parse_next(&mut tokens)?;       // commit: "expected `=>`" otherwise
```
