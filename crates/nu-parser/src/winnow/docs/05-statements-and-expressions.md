# 05 Statements and expressions

Files: `parse_expressions.rs` (dispatch, shorthand, assignments, math), `parse_keywords.rs` (what
keyword statements share), `parse_calls.rs` (calls), and one file per group of statements:
`parse_def.rs`, `parse_bindings.rs`, `parse_alias.rs`, `parse_type_alias.rs`, `parse_module.rs`,
`parse_source.rs`, `parse_control_flow.rs`. All are in `src/winnow/syntax/parser/`.

The input is a `Tokens` stream over the items of one command and its `Position`. The output is one
`Expression`. "Recovered" below has the meaning chapter 03 gives it, with its three forms: garbage
leaf, call form, node as written. A call form for an error only a signature reveals
(`MissingPositional`, `ExtraPositional`, `UnknownFlag`) records nothing. Any other records a
recovered diagnostic.

## Dispatch: `parse_expression`

`Position::Statement` is a command that is a whole pipeline. `Position::Element` is an element of a
longer pipeline, the left side of an assignment, an `else` branch or a match arm. `parse_pipeline`
collects every command before parsing any so it knows which applies. The order matches nu-parser's:

1. In statement position, a statement keyword (`is_statement_keyword`: `def`, `extern`, `let`,
   `mut`, `const`, `for`, `alias`, `type`, `module`, `use`, `export`, `export-env`) goes straight to
   `parse_builtin_commands`. This comes before the assignment test because `let x = 1` contains an
   `AssignmentOperator` token.
2. `parse_env_shorthand_prefix` reads leading `NAME=value` items.
3. After shorthand, or as an element, `check_builtin_command_in_pipeline` checks the head against
   nu's table. A hit is recovered and the head is read as a call.
4. Any assignment operator among the items makes it `parse_assignment_expression`. The exception is
   a statement-position head that nu's `parse_builtin_commands` reads by keyword
   (`is_builtin_keyword_head`: `overlay`, `source`, `source-env`, `run`, `hide`, `where`, `plugin
   use`), so `overlay use x = y` is a call.
5. A first item that passes `is_math_expression_like_at` makes it `parse_math_expression`.
6. `run` anywhere, and `source`, `source-env` and `hide` at a statement's start, are picked by their
   first word (`is_first_word_command`, `parse_first_word_call`), so `source me` is a `source` even
   where a command `source me` exists.
7. A `let` with a longer known name (`def "let me"`) is a call.
8. Anything else goes to `parse_builtin_commands`.

The result is wrapped in `EnvShorthand` when there were assignments.

**`is_math_expression_like`** is true for `true`, `false`, `null`, `not`, `if`, `match`, items
starting with `( { [ $ " ' -` or `r#`, and items whose text is a number, unit, datetime, binary or
range whose bounds parse. It is a function of the text, so `-1 | math abs` is math, `"ls"` is a
string not a call, and `1 + 1` at a pipeline head is arithmetic, with nothing parsed twice. A `$`
bound must be a variable name and a valid cell path, so `..$x.c!!` is an external command.
`is_math_expression_like_at` also parses a range with a `{` in its text, because the text doesn't
tell whether its record parses (`..{a: =}` is an external command).

**`check_builtin_command_in_pipeline`** refuses, in an element or after shorthand: `def`, `extern`,
`alias`, `type`, `for`, `module`, `use`, `source`, `hide`, `export` and `export-env`
(`BuiltinCommandInPipeline`); `const` and `mut` (`AssignInPipeline`); `overlay` unless the command's
second item is `list`; and `plugin` when its second item is `use`. So `FOO=1 def x [] {}` and `hide
ls | length` are refused, while `ls | overlay list` parses. The second item counts from the start of
the command, shorthand included. The call that results may be an external one, when an alias of an
external command is named for a statement (`alias "hide me" = ^echo hi`, then `[1] | hide me`). An
element with an assignment operator is an assignment first, with its left side checked the same way
(`ls | mut x = 1`).

`let` isn't in the table. `ls | let x` parses, and `ls | let x = 1` is an assignment whose left side
isn't a variable, because after a `|` the keyword isn't a statement. `while` and `loop`, which have
no output, are reported in a longer pipeline by `check_loop_in_pipeline`, at the first item of each
element after any shorthand, recovered.

## Keyword statements: `parse_builtin_commands`

`parse_builtin_commands` matches the head text and calls one function per keyword. Anything else is
a call. It also attaches the grammar context (`while parsing for`) to any error. A non-statement
keyword (`if`, `loop`, `where`, ...) that a user command shadows is a call. So is `if`, `match`,
`while`, `loop`, `try`, `return`, `break` or `continue` when a longer known command name starts with
it: `def "if ready" [] {}` makes `if ready` a call, because nu parses these with `parse_call` and
its longest match finds the longer name.

| File | Functions |
| --- | --- |
| `parse_def.rs` | `parse_def`, `parse_extern`, `parse_for`, `parse_def_predecl` |
| `parse_bindings.rs` | `parse_let`, `parse_mut`, `parse_const` |
| `parse_alias.rs` | `parse_alias` |
| `parse_type_alias.rs` | `parse_type_alias` (chapter 07) |
| `parse_module.rs` | `parse_module`, `parse_use`, `parse_export_in_block`, `parse_export_env` |
| `parse_source.rs` | `parse_where` |
| `parse_control_flow.rs` | `parse_if`, `parse_match`, `parse_while`, `parse_loop`, `parse_try`, `parse_return`, `parse_break_or_continue` |
| `parse_keywords.rs` | `is_statement_keyword`, `ALIASABLE_PARSER_KEYWORDS`, `UNALIASABLE_PARSER_KEYWORDS`, `KeywordCall`, `keyword_boundary`, `parse_help_call`, `parse_block_argument` |

Each keyword function follows one pattern: `KeywordCall::start` consumes the keyword,
`call.positional` takes each positional item after looking for flags, items go to `parse_value` with
the right `ExpectedShape` or to `parse_block_argument` for a block, and `call.end` and `call.finish`
close the statement. Shortened from `parse_for`:

```rust,ignore
pub fn parse_for<'a>(mut tokens: Tokens<'_, 'a>) -> ParseResult<Expression<'a>> {
    let working_set = tokens.working_set;
    let mut call = KeywordCall::start(&mut tokens)?;
    let Some(variable) = call.positional_word(&mut tokens, "loop variable")? else {
        return call.help_call();
    };
    let (var, typed) = /* parse_var_with_opt_type; a name nu refuses is call.call_after(..) */;
    let ty = /* if typed: tokens_until("in"), every item before `in` as one type */;
    let Some(in_keyword) = call.positional_word(&mut tokens, "`in`")? else {
        return call.help_call();
    };
    if tokens.text(&in_keyword) != "in" {
        return call.call_after(Diagnostic::new(ErrorKind::ExpectedKeyword("in"), in_keyword.span));
    }
    /* the iterable, parsed with parse_value; the block, with call.positional and parse_block_argument */
    call.end(&mut tokens)?;
    call.finish(Expression::new(Expr::For(Box::new(for_loop)), span))
}
```

Fixed shapes use the token parsers and winnow's combinators. `if` splits at `else` with
`tokens_until("else")` and `opt(keyword("else"))`. `try` reads a handler with ``expected("`catch` or
`finally`", alt((keyword("catch"), keyword("finally"))))``.

The keyword isn't stored in the tree. `Expr::keyword()` names it and `Expression::keyword_span()`
locates it, since it is always the first word of the expression's span.

### Flags at positional boundaries: `KeywordCall`

Every keyword is a command with a fixed signature in nu, so `parse_internal_call` looks for flags at
the start of each positional argument. `KeywordCall` reproduces that (`start`, `start_positional`,
`flags`, `call_flags`, `positional`, `positional_word`, `missing`, `end`, `is_call`, `help_call`,
`call_after`, `finish`):

| At a boundary | Effect |
| --- | --- |
| `--help`, `-h`, `--help=value` | The statement is an ordinary call showing help. Parsing goes on and only *missing* positionals are forgiven: `match 1 --help :{}` still has no match block, `match 1 --help` is fine. `finish` makes the call through `parse_help_call`, which re-parses the items with `parse_call`, so `def foo [] --help` is a `Call` to `def` with three arguments. A `--help=` value is read as a Boolean by the lowering (`if --help=1` is "expected bool") |
| The first `--` | Consumed and recorded in `Ast::ignored` (`return -- 1` returns `1`). After it nu looks for no flags at all: a second `--` is a positional, and `return -- --help` returns the string `--help` |
| Any other `-x` | A flag the keyword doesn't have, `UnknownFlag` (`return -1`, `match -1 {}`, `if -1 > 0 {}`). Call form, nothing recorded |
| A positional missing at the end | `MissingPositional`, call form through `call.missing`, nothing recorded (`loop`, `if true`, `while true --`, `match 1`, `def foo [x]`, `use`, `[1] \| where`) |
| An item after the last positional | `ExtraPositional`, call form, nothing recorded (`break x`, `loop {} y`, `return --help 1 2`) |

Where a positional spans several items (a condition, a signature, an alias target) only its first
item is a boundary. A lone `if` or `match` isn't a call: nu reads it as a math expression missing
its operand, "expected expression" with garbage, recovered (`lone_keyword`).

The statements nu parses by position never see `--`. `export-env` starts its `KeywordCall` with
`start_positional`, and `let`, `mut`, `const`, `alias` and `module` call `keyword_boundary_with(..,
end_of_options = false)` and `parse_help_call` directly. So `alias -- x = ls` and `let -- x = 1` are
errors, and `module x -- {}` and `export-env -- {}` are call form, whose errors the lowering
reports.

nu-parser computes the extent of a multi-item argument from the signature: `if`'s condition gets all
spans up to the ones the remaining required positionals need. Here each keyword spells out the rule,
with the same result and no signature. The `if` condition is the `tokens_until("else")` stream minus
its last item, `while` splits at the last item, and the `def` signature is every remaining item but
the last.

### `def` and `extern`

```text
def [--env] [--wrapped] NAME SIG[:] [TYPES] BODY
extern [--strict] NAME SIG[:] [TYPES]
```

**Name.** `parse_def_name` wants a string item. `check_definition_name` refuses parser keywords,
names with `#`, `^` or `%`, names whose first word reads as a value (`is_literal_like`: `0x10`,
`1kb`, `2020-01-01`, `1..2`; words like `1k`, `0xZZ` and `infinity` are fine), and whitespace other
than single spaces between words. These are recovered and the predeclaration declares nothing for
them (`recover_name_error`). Other names nu's call refuses are call form:

* `def $x`, `def $"x"`, an open quote `def "f`, a spread `def ...{x: 1} [] {}`.
* A name with `[` or `(`, even quoted, "no space between name and parameters"
  (`signature_error_call`), which the lowering refuses as nu's `detect_params_in_name` does.
* A bare name starting with `-`, which is no name. A lone `-` is the name for nu's call and no item
  for its predeclaration, which takes the next item not starting with `-` (`decl_name_span`,
  `params_in_dash_name`). So in the shell `def - [] {}` reports the params-in-name error, and `def -
  foo [] {}` doesn't read its signature.

**Flags.** `parse_definition_flags` is a loop over `definition_flag`, looked up in `DEF_FLAGS` or
`EXTERN_FLAGS`. Flags may also follow the name and may take a value (`--env=false`, `--wrapped=$c`,
`--strict=false`), kept in `Def::flags` and `Extern::flags`. An empty value is a garbage point,
recovered (`parse_flag_value`). `DefinitionFlag::is_set` reads them as nu's `has_flag_const` does:
the first of a repeated flag decides, and a value other than `true` or `false` needs constant
evaluation, so checks that depend on it are the lowering's. The loop also takes help flags (`def
--help --env f`). An unknown or short flag before the signature, or a flag or `--` in the body's
place, makes the statement call form (`reads_as_call`).

**Signature.** `parse_full_signature` takes every remaining item but the last. One item is the
signature. Two of which the second starts with `{` are the signature plus an item nu drops (`def foo
[] {} {}`, the dropped one in `Ast::ignored`). Otherwise types follow a `:`, attached or standing
alone (`def foo [] : {}`), re-lexed together in signature mode. A signature it refuses (a name taken
twice, a bad parameter name or type, a default of the wrong type) is call form, because nu reads the
signature as its call's argument, reports its errors and still defines the command. A positional
there that doesn't parse as a value (`[]: int`, `[]{}`) is a garbage leaf (`parse_definition_call`).

**Body.** `parse_def_body` parses a closure without a probe: `{|x| }` keeps its parameters as
`Def::body_params`, and `{a: 1}` is a call to `a:`. A body that is no closure (`def a [] (echo 4)`)
is call form.

**`--wrapped`** needs a rest parameter that is untyped or `string`, which is `finish_def`'s check in
the lowering. The predeclaration takes only the item `--wrapped` as written for a wrapped command. A
wrapped command's untyped rest parameter gets nu's `external_arg` shape (below).

**`extern`** differs in that its signature takes *every* remaining item, so a body (`extern foo []
{}`) is the dropped second item, and default values aren't parsed (chapter 07). The lowering makes a
call with an undeclared flag an error under `--strict`.

**Help and predeclaration.** The help call of `def foo [] {}{} --help` leaves out the dropped item
(`help_call_without`). The predeclaration reads items as nu's lite parser leaves them, so a
redirection without a target and every item after a redirection are items: `def f [] { } o> o> x`
predeclares from `[] { } o> o> x`.

### `let`, `mut`, `const`

```text
KW NAME[:] [TYPE...] [= rest]
```

nu reads a binding only with four items or more and a plain `=` after the name with a value after
it. Anything else is the call to the keyword. The call form reads an `=` as any item (`call_item`).

* `let = 1` misses the name (`MissingPositional`), `let x =` the value (`KeywordMissingArgument`),
  and `let x += 1` the `=` (`ExpectedKeyword`). All are call form.
* In a binding the first item is the name, even a flag. `let --help x = 1` has no variable name,
  recovered, and the items before the `=` are extra tokens.
* Without an `=` it is an ordinary call, so a `--` before the name ends the options (`let -- x`,
  `const -- x`). With one, `--` is a name and refused. The value is optional for `let` and a missing
  positional for `mut` and `const` (`const x`). A typed `mut x: int` or `const x: int` without a
  value is `MissingType`, because of the two required positionals nu gives the variable only the
  `x:` item (`calculate_end_span`).

The name may start with `$`, and a bare `$` is a name. In a binding nu reads, these are recovered,
the node kept (`variable_declaration`, `binding_variable`):

| Input | Error | Result |
| --- | --- | --- |
| `let in = 1`, and `nu`, `env`, `ans` | `NameIsBuiltinVar` (`ensure_not_reserved_variable_name`) | variable declared |
| `let a int = 1`, `let a : int = 1` | `ExtraTokens` | extra items in `Ast::ignored` |
| `let 'a b' = 1` | `VariableNotValid` | variable is garbage |
| `let $x-y = 1`, `let -- x = 1` | `Expected` | variable is garbage |
| `let a: = 1` | `MissingType` | |
| `let $x.y: int = 1` | a typed name nu refuses | type read |
| `let 'a b': int = 1` | a typed name with a blank or quote | type is extra tokens |

Type tokens are re-lexed together (`parse_type_after_var`). A type of several items is lexed as a
signature first (`parse_var_type`), so `record<a: int, b]>` is unbalanced. The value is the rest of
the line, parsed as a *block*, so it is a whole pipeline.

### `for`

```text
for VAR[:] [TYPE] in ITEM BODY
```

* A typed variable takes every item before `in` as its type: `for x: record<a: int, b: string> in []
  {}` parses, and `for x: int --help in [] {}` has the unknown type `int --help`.
* Call form: no type (`for x: in [] {}`), a name nu refuses (`for x.y in [1] {}`), and a word in the
  place of `in` (`ExpectedKeyword`). An `=` reads as any item in each place (`for l = [1]`,
  `KeywordCall::positional_word`).
* The argument of `in` is never forgiven. nu reserves the last item for the block before it reads
  that argument, so `for x in []` and `for x --help in []` are both `KeywordMissingArgument`, call
  form. After a keyword without its argument nu's call skips the next item, so `for x in [1]` misses
  its block. The lowering finishes a `for` call with `finish_for`.

### `alias`

```text
alias NAME = rest
```

nu checks the layout by position first (`check_alias_name`). With three or more items after `alias`,
the second must be `=`. With one or two, the first must not be. A misplaced item is recovered, the
statement the alias call (`call_after_error`).

* **Fewer than three items** (`alias x=y`, `alias x`, `alias x ls`, `alias x =`, also after `export`
  and with an unknown flag) keep nu's call to `alias`, which declares nothing and reports it as
  incomplete (`IncorrectValue`). Call form through `parse_alias_as_call`, nothing recorded, checked
  by the lowering with nu's `alias_name` and `report_incomplete_alias`.
* **Without a usable name**: `alias` alone (`UnknownState`), an open quote (`alias "f`), or a name
  nu refuses after reading the call (`$x`, `(x)`, `{x}`, a parser keyword), all recovered and no
  alias declared. A bare name starting with `-` is "alias name not supported", though a lone `-` is
  a name.
* **Help**: `--help` or a short batch with `h` before the `=` is a help call (`alias --help = ls`,
  `alias -hx`). It must be the whole statement, so `alias --help x = ls` is "missing sign". `alias`
  and `type` drop the errors of the call nu parses for them.

The target is checked in nu's order. Each failure is recovered, and nu declares no alias:

1. A target that `is_math_expression_like` (a literal, `$x`, `(..)`) is "cannot create an alias to
   an expression" (`CantAliasExpression`), the target a garbage leaf the lowering reads with its
   port of `parse_expression`. The exception is `0..<$nope`, whose first item nu doesn't read as an
   expression after all: the target is a call to an external command, and the alias is declared.
2. A `|` or redirection at the top level of the target (`alias x = ls | length`) is "cannot create
   an alias to a pipeline" or "to a redirection". A quoted one, or one inside `( )`, is part of a
   word. The target reads as words (`pipe_or_redirection`).
3. A target naming an unaliasable keyword (`UNALIASABLE_PARSER_KEYWORDS`: `def`, `let`, `for`,
   `export def`, ...) is "cannot create an alias to a parser keyword", the target read for
   `finish_alias`. `if`, `match`, `try` and `overlay *` may be aliased.

The rest goes to `parse_call_lenient(.., true)` as plain words, so `alias x = FOO=1 ls` calls the
external `FOO=1`. Lenient means a keyword-command target may miss positionals and flag values
(`alias x = overlay new`). An aliased `if`, `match` or `try` is parsed with the keyword's own rules,
forgiving only missing positionals: `alias i = if x` is fine, `alias i = if x y` is not.

nu's first reading of the target (`parse_expression`) reads `run` and `where` by their first word
(`parse_run_expr` parses the script in its own scope) and reads a target with an assignment operator
as an assignment (`alias d = let x = 1`). The lowering gets this by reading the target again as an
expression.

An alias is declared once its statement is parsed. A call to an alias of an external command is an
external call. In a module's body, an `export alias` that defines no alias is nu's `InternalError`,
reported by the lowering.

### `module`, `use`, `export`

```text
module NAME [BODY [--]]        use MODULE [MEMBER...]        export def|extern|alias|use|module|const ...        export-env BODY [ITEM...]
```

**`module`.** A declaration is a literal string name (`parse_module_name`), then a block or nothing.
One `--` after the body is dropped. A body holds declarations, or a call for help of one of their
keywords (`def --help`). Any other statement nu reports at its first item (`ExpectedKeyword`) and
leaves unread: recovered, garbage. Anything else is the call nu makes to `module` first
(`parse_module_call`, nu's `ArgumentParsingLevel::FirstK`): flags up to a `--` and spreads as in any
call, the first positional parsed, every later one `Argument::Unread` (`module x {ls} --help`). A
call for help, an unknown flag, a missing or extra positional, or a name that is no literal are that
call's errors. The lowering goes on with the item after the name as the body, so `module x {} --foo`
adds the module and `module x --foo {}` is "expected block".

**`use`.** `MODULE` is a string or `null`, a no-op after which members are parsed and then ignored.
Members (`parse_import_pattern_member`) are names, `*`, or a `[a b]` list
(`parse_import_pattern_list`), and only the last may be `*` or a list. A variable, subexpression or
`key: value` record is kept as `ImportPatternMemberKind::Ignored`. Non-string list items and a cell
path after a list (`[math].x`) go to `Ast::ignored`, and anything else is "wrong import pattern
structure". nu reports a wrong member and drops it, and a member after a `*` or list ends the
pattern, each going on with the import, so these are recovered with every member kept, and the
lowering reads the pattern with its port of `parse_import_pattern`. A `use {a: 1}` record module is
recovered too. Flags are looked for before every argument up to the first `--`, which is dropped
(`Use::end_of_options`). Every item after it is a member, so `use std -- --help` asks for no help.

**`export`** wraps the inner statement (`parse_export_in_block`). An export of nothing (`export`,
`export foo`) is call form. `export --help x` has an extra positional.

**`export-env`.** nu hands it exactly one argument, so extra items (`export-env {} extra`) and a
redirection on it are `Ast::ignored`, and after a help flag the rest is ignored text (`export-env
--help x {}`). A closure or record body is refused. A lone `export-env` is reported with garbage. An
`export-env {` with its block open gives its block, which a module's body adds
(`finish_export_env`).

### Control flow

| Keyword | Shape | Notes |
| --- | --- | --- |
| `if` | `if COND... BLOCK [else BLOCK\|EXPR]` | The condition is every item before the block, and the block is the item before `else` or the last item |
| `match` | `match ITEM BLOCK` | Chapter 07 covers the block |
| `while` | `while COND... BLOCK` | As `if`. A `$` or `(...)` body is `While::body_value` |
| `loop` | `loop BLOCK` | |
| `try` | `try BLOCK [catch\|finally CLOSURE]{0,2}` | Handlers stay in source order, and nu allows two of either kind |
| `return` | `return [ITEM]` | One item only |
| `break`, `continue` | keyword alone | |
| `where` | `where {closure}` or `where COND...` | A row condition, `parse_condition` |

**`if` block.** A `$` or `(...)` item where the block belongs is kept as `If::then_value`, because
nu type-checks it (`if $c $env.f`). The bodies of `while`, `loop`, `try` and `for` work the same way
(`body_value`). A `key:` record there is "expected block" when nu would type it `record<...>`,
meaning every key is a plain string. nu types any other record, and a cell path on one, `any`, which
it accepts as a block (`if true { $env.A:b }`, `if true {a: 1}.a`). `parse_brace_value` keeps those
as the value. An error in the condition or the block that nu's call goes past (`if true 1`, `if
--help true {} x`) is call form, for `if` and `while`.

**`if` help.** The condition takes every item but the last. When that last item is `--help`, the
statement is a help call once the condition parses (`if x --help`, but not `if if --help`).

**`else`.** The branch is a block or a whole expression (`parse_block_or_value`, or
`parse_expression` in `Position::Element`), so `else if ...` recurses and `else {|x| }` is a
closure. nu's `else` argument tries a block first from one item, which `parse_value` reads by its
first byte for a `$`, `(` or `r#` item. Such an item followed by more items is the argument alone,
and the statement is nu's call (`one_item_else`): `else $x $x` has an extra positional, and `else (1
+) 2` drops the `2`. An else branch of one block followed only by flags leaves them to `if` (`if c
{} else {} --help` is a help call, `... else {} --` drops the `--`). An `else` without its argument
is call form.

**`match`.** nu decides what the `{ ... }` is before it knows it wants arms, so a closure, a record,
a `$x` or a `(..)` in that position is accepted and kept as `Match::value_block`, with empty arms.
`match 1 [a]` and `match 1 foo` are recovered, the item a garbage `value_block`.

**`try`.** A handler is a closure, or a `$x` or `(..)` that may hold one (`parse_try_handler`). A
record is refused unless nu would type it `any`, as for the block of `if`. An item that is neither
keyword (`ExpectedKeyword`), a keyword without a value (`KeywordMissingArgument`) and a value that
is no closure are call form, whose `oneof` the lowering reads as `parse_oneof` does. After a first
`--` nu looks for no more flags, so in `try {} -- catch {} --` the second `--` is a positional.

**`where`.** `where --help` is a help call. One `{` item is a closure when it ends with `}`. nu
tries a closure first and falls back to a row condition, so `where {}.a` is a cell path on a record,
and a closure whose body names `$it` reads again with `$it` declared (`where {|x| $it > 1}`, in the
lowering's `row_condition`). Other commands taking a row condition parse their arguments the same
way (`DeclKind::RowCondition`).

### Redirections on statements

`rejects_redirection` refuses one, in a pipeline of that one command, on `def`, `extern`, `for`,
`alias`, `type`, `module`, `use`, `export`, an attribute block before a definition or an `export`,
any call whose head word is `overlay` (by name, before its arguments are looked at), and calls whose
keyword signature isn't `redirectable` (`hide`, `source`, `run`, `plugin use`). It is recovered, and
the lowering reports it with nu's name for the statement. Before a definition nu reads the
attributes first and keeps them over the garbage. An `export` it refuses before reading them, except
in a module's body, where it reads them and drops the redirection. In a longer pipeline nothing
refuses a redirection.

A redirection on `export-env`, a lone `const` or attributes before no definition is dropped and
recorded in `Ast::ignored`. A lone `let` or `mut` reads the operator and target as its own items, as
`parse_let` reads `parts_including_redirection`: `let a o> f = 1` has extra items before its `=`,
`let x o> f` has a word where its `=` goes, and the statement has no redirection.

### Attributes

`parse_attribute` reads `@name args` lines as calls to `attr name`. Whether `attr name` exists is
the lowering's business, since it may come from a `use`d module. An empty name (`@`, `@ name`) is
included, reported as an unknown command. The built-in attributes (`category`, `complete`,
`deprecated`, `example`, `interactive`, `search-terms`) get their arguments checked against their
keyword signatures.

Attributes may only precede `def`, `extern`, `export def` and `export extern`. Before anything else
they are "attributes must be followed by a definition", recovered, kept as an attribute block with a
garbage item that the lowering reads as `parse_attribute_block` does. Before an `export` of no
definition (`export module y {}`), `parse_export_in_block` does the same and then reports the
attribute block, which is no call, from the statement's second item on (`exported_attribute_block`).
A definition nu reads as its keyword's call (`@example ... def f [a, a] {}`) keeps its attributes.

## Math expressions: `parse_math_expression`

Operands are single items read with `parse_value(.., ExpectedShape::Any)`. Operators are items whose
text is an operator spelling. nu-parser folds them with an operator-precedence stack. Here winnow's
Pratt parser does it:

```rust,ignore
expression(parse_math_operand).infix(infix_operator).parse_next(&mut tokens)

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

winnow hands `fold_binary_op` only the two operands, so it finds the operator as the token right
after the left operand (`tokens.token_after(lhs.span.end)`). Precedence lives on
`ast::Operator::precedence`, with nu-protocol's values from 100 for `**` down to 40 for `or`. `**`
is the only right-associative operator.

* **`not`.** `parse_math_operand` reads prefixes with `repeat(0.., keyword("not"))`. With them, the
  operand is everything up to the next operator that binds looser than `not` (`NOT_PRECEDENCE`, 55:
  `and`, `xor`, `or`), found by `negated_operand_end` and parsed as a math expression of its own. So
  `not $a == $b` is `not ($a == $b)` and `not $a and $b` is `(not $a) and $b`.
* **Negation.** A value `-$x` or `-(expr)` is `Expr::UnaryMinus` (`parse_negation`), for the shapes
  that hold numbers.
* **`if` and `match`** are allowed as operands and consume the rest of the items (`1 + if $x { 2 }
  else { 3 }`).
* **Unknown operators** get targeted help from `parse_operator`: `^` suggests `**`, `%` suggests
  `mod`, `contains` suggests `has`.
* **Errors.** An item where an operator goes that is none (`1 2`) and a missing operand (`1 +`, a
  lone `not`) are recovered, and the expression is a garbage leaf over its items. The lowering reads
  it again with its port of nu's math parser, from the leaf's text less the element's redirections,
  which nu's lite parser takes out of the command (`"a" o> f b` reads `"a" b`). That happens only
  when the text lexes to the same items, which excludes an alias's words and items around a
  redirection nu refuses. Otherwise the error stays.

```rust
use nu_parser::winnow::syntax::{parse, ast::{BinaryOp, Expr, Operator, Math}};

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

`where` parses its condition with `parse_row_condition`: the math expression is parsed, then
`expand_row_condition` rewrites the finished tree. Any string operand on the left of an operator, on
either side of `and`, `xor` and `or`, or alone, becomes a cell path on the implicit `$it`
(`expand_to_cell_path`). So `where size > 1kb` yields `FullCellPath { head: Var it, implicit_head:
true, tail: [size] }`. Both names are columns in `where a and not b`, and in `where a == not b`,
where the rewrite goes into the right-hand `not`. Otherwise the right operand stays a value (`where
name == foo`).

```rust
use nu_parser::winnow::syntax::{parse, ast::Expr};

let ast = parse("ls | where size > 1kb").unwrap();
let Expr::Where(where_) = &ast.block.pipelines[0].elements[1].expr.expr else { panic!() };
let Expr::BinaryOp(condition) = &where_.condition.expr else { panic!() };
let Expr::FullCellPath(path) = &condition.lhs.expr else { panic!() };
assert!(path.implicit_head);
assert_eq!(path.tail.len(), 1);
assert!(matches!(condition.rhs.expr, Expr::Filesize(_)));   // the right operand stays a value
```

## Assignments

`parse_assignment_expression` splits the items at the first `AssignmentOperator`. The left side is
parsed in `Position::Element` and must be what `parse_full_cell_path` produces: a `Var`,
`Subexpression`, `FullCellPath`, `List`, `Table` or `key: value` `Record`, with or without a cell
path. So `(1) = 2` and `[1].0 = 2` pass here and fail at run time. Anything else is "assignment
requires a variable", recovered, node kept. The lowering also reports an immutable variable and an
operator the types refuse.

The right side, everything absorbed to the end of the line, goes to `parse_block` and is stored as a
`Block`, so `$x = ls | length` assigns the pipeline's result. A missing right side (`$x =`) is
recovered, an empty block after the operator with the left side unchecked.

An external command at the start of the value must have a caret: `$x = git` is "external command
calls must be explicit in assignments", recovered, and `$x = ^git` is fine. A bare external head
needs a command table or an alias of an external command, so with no table this never fires.

## Calls: `parse_call`, `find_longest_decl`

`find_longest_decl` implements nu's longest match: it tries the first *n* words joined with spaces
against the known commands, longest first, so `str trim --left` is a call to `str trim` with one
flag. With a `ParseConfig` it tries up to five words, and with a `CommandLookup` any number. It
returns the `DeclKind` the lookup gave for a name of several words. For a single-word head it
returns `None`, having looked nothing up, and the caller asks `find_decl`. The single-word path
builds no string, and a multi-word attempt happens only if `is_decl_name_prefix` says the first word
starts a known multi-word command. `find_longest_name` is the search itself, taking the question to
ask of each candidate: `find_longest_decl` asks `find_decl`, and the `%` sigil asks
`find_builtin_decl`.

### Arguments

`parse_call_arguments` is `repeat_to_end(parse_call_argument)`, so every item must be an argument:

| Item | Result |
| --- | --- |
| `--` | `Argument::EndOfOptions` |
| `--name`, `--name=value` | `Argument::Named`, `long: true`. An empty `--name=` value is a garbage point, recovered |
| `-x`, `-abc` | `Argument::Named`, `long: false`, unless it looks like a negative number (`-1`, `-.5`) |
| `...$x`, `...[..]`, `...(..)`, `...{..}` | `Argument::Spread` |
| anything else | `Argument::Positional(parse_value(.., ExpectedShape::Any))` |

`Argument::Unread` is never read from an item. `parse_module_call` puts it in place of each
positional after the name.

**By the command's parameters.** A call to a command whose parameters the parser knows is read by
them (`parse_call_arguments_by_shape`, nu's `parse_internal_call` for one-item shapes). A `def` or
`extern` of the text has its own signature's, in the shell too. Any other command has, in the
shell, the engine's signature (`CommandLookup::call_shapes`), unless it is a keyword, an alias, or
has a parameter no single item holds (a `block`, a row condition, a plugin's custom type), and
alone, the generated table's (`BUILTIN_SIGNATURES`). A call to any other command is read as above.

* After `--`, every item is a positional.
* A long flag that takes a value takes it after `=` or from the next item (`NamedArgument::equals`
  tells which), read by the flag's shape. A switch's `=value` is a bool.
* In a batch of short flags, the last letter may take the next item.
* An item starting with `-` is flags unless none of its letters is a flag and the positional's shape
  takes a negative number. `-5` works for an `int`, while `echo -5` is an unknown flag.
* Each positional is read by its parameter's shape (`ExpectedShape::Declared`), `any` past them. A
  value with an error nu goes past is a garbage leaf, recovered, so `ls 0b2` is a glob and `f 0b2`
  is an error for an `int` parameter.

Unknown flags and missing or extra positionals stay the consumer's. Alone, after a `use`, `overlay
use` or `source` in a scope (`note_import`), configured commands are read without their signatures:
the parser loads no module, and an imported command may extend a configured one or shadow it.

**In the shell,** the lowering assigns the arguments by the real signature. A flag the parser read
with the next item as its value it takes as nu's reader meets a flag and the item after it
(`split_flag_value`). It gives each multi-item positional its items as `calculate_end_span` does
and reads them as `parse_multispan_value` does (`lower/call.rs`, `multispan_value`). A row
condition or math expression of several items it has the parser read again from the items' text
(`parse_items`).

### Keyword commands that are calls here

`hide`, `source`, `source-env`, `run`, `overlay new`, `overlay use`, `overlay hide`, `overlay list`,
`plugin use` and the built-in `attr *` attributes have fixed signatures in nu and are plain calls
here. An unknown flag, a missing or extra positional or a flag without its value is the lowering's
error. `parse_call_lenient(tokens, lenient)` finishes by calling `check_call` for the two parts nu
reads as syntax, counting positionals as nu does (a flag before a `--`, a word like `-1` there and a
spread are none):

* `hide`'s second positional is an import pattern (`hide foo null` is wrong), unless a `--help` or
  `-h` before any `--` makes the call show help, or `lenient` is set.
* `overlay use`'s `as NAME` is a keyword argument after the module. The positional after the module
  must be `as` (`overlay use x -- --y` is an error), and any item after it is its value (`as
  --prefix`). With no command table the head of `overlay use x` resolves as `overlay` with `use` as
  its first argument, and `keyword_signature_of_call` finds both spellings.

### The `%` sigil

`parse_percent_call` forces the built-in command even when a custom command or alias shadows the
name. `%ls` and `% ls` give a `Call` whose `sigil` is the span of the `%`. `%$cmd` and `%(expr)`
give a `DynamicCall`. The head is the longest name of a built-in command, visible or not
(`find_builtin_decl`), so `%ls foo` calls `ls` with the argument `foo` even where `def "ls foo"`
exists. Arguments read by the command's kind, so `%any $it > 2` takes one row condition.

A quoted name after `%` is an error. A bare name that isn't a built-in (`CommandType::Builtin`, so
no keyword like `if` and no prelude command like `pwd`) is "percent sigil requires a built-in
command", skipped when the parse knows no commands, and points at the first word. nu then reads the
items after the `%` as an external call, so the parser records it as recovered and reads that
(`ExternalCall` with the `%` as its `caret`). After a lone `%`, nu's head is the empty rest of that
item: a following item that names no built-in, or none, makes an external call with an empty head
and every item after the `%` as arguments.

## External calls

`parse_external_call` reads `^cmd args`. The head after `^` is a string, `$var` or `(subexpr)`.
Arguments are `repeat_to_end(parse_external_call_argument)`: `...$x`, `...[..]` and `...(..)` are
spreads, `$..`, `(..)`, `[..]` and `{..}` are values, and everything else is an external string. A
`[..]` argument must end with `]` because nu hands it to its list parser alone, so `^cmd [a].x` is
"unclosed delimiter", recovered as a garbage leaf, while `^cmd (ls).name` parses.

Three other paths reach an external or external-style call:

* A bare head that resolves to no command, with a command table configured. `parse_call_lenient`
  hands it to `parse_external_arguments` with `ExternalCall::caret` set to `None`, so `git log
  0b2d1f4..HEAD` has three external strings.
* A call to an alias of an external command (`DeclKind::ExternalAlias`).
* A `def --wrapped` with an untyped rest parameter (`DeclKind::Wrapped`, recorded by
  `parse_def_predecl`, whose `has_untyped_rest` takes the rest parameter from the lexed signature
  and looks for its type on the text, like nu's `rest_param_is_type_annotated`). Every argument
  reads as external, so `f 'x'$` and `f 0b2` are strings, and the call records it (`Call::wrapped`).
  nu gives any positional before the rest its own shape, which the lowering does.

`parse_external_string` reproduces nu's segmenting: a word splits into bare, quoted, backtick and
parenthesised segments (the `ExternalStringSegment` state machine, a byte loop), quoted segments
stay literal, parenthesised bare segments interpolate, and all-literal words collapse to one string.
So `--query='q($x)'` keeps its parentheses and `--out=(pwd)/x` interpolates.

Two more kinds change how a call parses. A `DeclKind::RowCondition` command takes its arguments as
one condition (`parse_condition`), so `any $it > 2` has one positional, a comparison. When the first
argument is a flag the parser can't tell whether nu takes the next item as its value (`take while
--include 2 ..`), and reads the arguments one by one. A `DeclKind::KeywordAlias` call (`alias m =
match`) is parsed by the keyword's own parser, with the alias's name in the keyword's place
(`If::keyword`).

## Environment shorthand

In `FOO=bar BAZ=$x cmd`, leading `NAME=value` items with an identifier `NAME`
(`is_env_variable_name`) become `EnvShorthand`. The value is a `$` expression or a strict string,
with no space outside double or single quotes, so `FOO=(1 + 1) ls` is the external command `FOO=(1 +
1)`. `parse_env_shorthand_prefix` is `repeat(0.., env_assignment)`, and the first item that isn't
`NAME=value` makes `verify_map` backtrack, ending the `repeat` at the command's head.

With no command after the shorthand (`FOO=1` alone), nu reports the first item as an unknown
command. It is recovered, the command a garbage point after the last assignment.

## Stream idioms you will reuse

```rust,ignore
let keyword = tokens.expect_item("for")?;               // next Item, or "expected for" at the right span
if tokens.at_end() { .. }                               // nothing left
tokens.expect_end()?;                                   // ExtraTokens error otherwise
let rest: Option<Span> = tokens.consume_rest();         // consume the rest, get its span
let condition = tokens.slice(start..end);               // a sub-stream over some of the items
let value = parse_value(tokens.working_set, token.span, ExpectedShape::Any)?;  // one item

// token parsers, composed with winnow combinators
let else_keyword = opt(keyword("else")).parse_next(&mut tokens)?;           // an item spelled `else`, or None
let before_in = tokens_until("in").parse_next(&mut tokens)?;                // a stream over the items before `in`
let arguments = repeat_to_end(parse_call_argument).parse_next(&mut tokens)?; // every item, or an error
let arrow = expected("`=>`", keyword("=>")).parse_next(&mut tokens)?;       // commit: "expected `=>`" otherwise
```
