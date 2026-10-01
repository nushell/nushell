# The Nushell grammar, as `nu-parser` parses it

This file is a BNF description of the Nushell language **as accepted by
`nu-parser`** (nushell `main`, commit `d69f33312`, 2026-09-30). Every rule was
derived by reading nu-parser's source, and the rules are annotated with the
function that decides them, so that a change to the parser can be traced to the
rule it affects and the other way round. The ```` ```ebnf ```` fences are the
machine-readable grammar: `grammar.bnf` and `grammar.ebnf` are generated from
them (section 8).

Nushell has no official grammar; the language is whatever `nu-parser` accepts.
Much of nu-parser is *shape directed*: the syntax of an argument position is the
`SyntaxShape` of the command's signature (`if`'s `else` branch is
`Keyword("else", OneOf[Block, Expression])`). This grammar therefore has two
kinds of rules: purely syntactic ones (decidable from the text alone) and
shape-directed ones, which need a command table, declarations, constant
evaluation or files. Where a rule is shape-directed, the comment says which
signature or table decides it.

## 0. Notation and reading the annotations

```ebnf
<name> ::= alt1 | alt2      a rule; terminals are in double quotes
[ x ]                       optional
{ x }                       zero or more
( x )                       grouping
1*x                         one or more; N*x exactly N
x - y                       x except y (ISO 14977 exception; used for name restrictions)
<words with spaces>         a class described in words, not a nonterminal
; ...                       a comment; prose describes character classes
                            and side conditions the notation cannot express
; nu: file.rs::fn           where nu-parser decides the rule: the function (or
                            const, enum, impl) in crates/nu-parser/src/file.rs,
                            unless another crate is named
```

Two facts about nu-parser shape everything below and are easy to forget:

1. **Items, not characters, are the unit.** The lexer splits source into
   *items*: maximal runs of non-whitespace text in which quotes and brackets
   balance. `[1 2 3]`, `{a: 1}`, `$x.a.0` and `foo(1 + 1)bar` are each one item.
   `1+1` is a bare word; `1 + 1` is three items and a math expression. Every
   value rule in section 5 describes the text of exactly one item. Bracketed
   items are re-lexed from their interior with construct-specific options
   (section 1.4).
2. **Speculation order is grammar.** Where nu-parser tries parsers in sequence
   (`1kb` is a filesize before it could be a string; `0x[zz]` is an error and
   never a string), the order is stated, because it decides what the text means.

## 1. Lexical layer

### 1.1 Layout

```ebnf
<source-file>     ::= [ <shebang> ] <token-stream>
; a leading "#!" line is an ordinary comment to the lexer; the lite parse (2)
; groups the token stream into <block-tokens>
; nu: lex.rs::lex_internal (comment at token start)

<whitespace>      ::= " " | "\t" | <carriage-return> | <additional-whitespace>
; <additional-whitespace> depends on the construct being lexed (1.4)
; nu: lex.rs::lex_internal (skipped between tokens), lex.rs::is_item_terminator (terminates an item)

<carriage-return> ::= "\r"
; ignored between tokens; inside an item it terminates the item like whitespace
; nu: lex.rs::lex_internal, lex.rs::is_item_terminator

<eol>             ::= "\n"
; an Eol token, unless "\n" is additional whitespace for the construct
; nu: lex.rs::lex_internal

<comment>         ::= "#" { <any byte except "\n"> }
; At a token boundary "#" always starts a comment running to "\n" (a "\r" before
; the "\n" is part of the comment). Inside an item "#" starts a comment only when
; the previous byte of the item is ASCII whitespace or there is none: `a#b` and
; `[a]#b` are not comments. A comment inside brackets ends at "\n"/"\r" and the
; item continues. Comments are dropped when the construct lexes with skip_comment.
; nu: lex.rs::lex_internal, lex.rs::lex_item
```

### 1.2 Tokens

```ebnf
<token-stream>    ::= { <token> }
<token>           ::= <item> | <pipe> | <pipe-pipe> | <semicolon> | <eol>
                    | <comment> | <assignment-op> | <redirection-op>
                    | <bashism>                      ; always an error
; nu: lex.rs::lex_internal, lex.rs::TokenContents

<pipe>            ::= "|"
; A "|" directly after an <eol> token replaces that Eol (line continuation), and
; ([Eol] Comment) pairs directly before it lose their Eol too, so `a\n# c\n| b` is
; one pipeline. Exactly ONE Eol is absorbed: `a\n\n| b` keeps an Eol (see
; <pipe-continuation>, 2).
; nu: lex.rs::lex_internal

<pipe-pipe>       ::= "||"
; an error (ShellOrOr) everywhere except as empty closure parameters
; nu: lex.rs::lex_internal, lite_parser.rs::lite_parse

<semicolon>       ::= ";"
; ";" directly after "|" is an error
; nu: lex.rs::lex_internal

<assignment-op>   ::= "=" | "+=" | "-=" | "*=" | "/=" | "++="
; an item whose whole text is one of these
; nu: lex.rs::is_assignment_operator, lex.rs::lex_item

<redirection-op>  ::= <out-redirect> | <err-redirect> | <out-err-redirect>
                    | <err-pipe> | <out-err-pipe>
<out-redirect>    ::= "o>" | "out>" | "o>>" | "out>>"
<err-redirect>    ::= "e>" | "err>" | "e>>" | "err>>"
<out-err-redirect> ::= "o+e>" | "e+o>" | "out+err>" | "err+out>"
                    | "o+e>>" | "e+o>>" | "out+err>>" | "err+out>>"
<err-pipe>        ::= "e>|" | "err>|"
<out-err-pipe>    ::= "o+e>|" | "e+o>|" | "out+err>|" | "err+out>|"
; an item whose whole text is one of these. The trailing "|" of the ">|" forms is
; glued on only when the text before it is one of "o>" "out>" "e>" "err>" "o+e>"
; "e+o>" "out+err>" "err+out>"; otherwise "|" ends the item.
; nu: lex.rs::lex_item, lex.rs::is_redirection

<bashism>         ::= "o>|" | "out>|" | "&&" | "2>" | "2>&1"
; items with exactly these texts are errors with a hint; the parse goes on with
; the token read as "|" / "|" / "|" / "e>" / "|" respectively
; nu: lex.rs::lex_item
```

### 1.3 Items

```ebnf
<item>            ::= 1*<item-part>
<item-part>       ::= <plain-byte> | <quoted-run> | <interp-quoted-run> | <raw-string> | <bracketed-run>
; An item runs until, at bracket depth zero, a terminator byte is met: " " "\t"
; "\n" "\r" "|" ";" or any byte in the construct's additional whitespace or
; special tokens (1.4). Quotes and brackets suspend termination. An empty item is
; UnexpectedEof("command"). A special byte at the START of an item is an item of
; its own (":" in record keys, "." in cell paths).
; nu: lex.rs::lex_item, lex.rs::is_item_terminator

<plain-byte>      ::= any byte that is not a terminator, not "'" "\"" "`", not
                      "[" "{" "(" "]" "}" ")", not "r" followed by "#", and
                      (only in signature mode) not "<" or ">"
; "#" is a plain byte unless it starts a comment (1.1)
; nu: lex.rs::lex_item, lex.rs::is_item_terminator

<quoted-run>      ::= "\"" { <dq-byte> } "\""
                    | "'"  { <byte except "'"> } "'"
                    | "`"  { <byte except "`"> } "`"
<dq-byte>         ::= "\\" <any byte> | <byte except "\"" and "\\">
; Escapes exist only inside double quotes. Newlines are ordinary bytes inside
; quotes. An unclosed quote is Unclosed. A quote may start mid-item (`foo"bar baz"`)
; and the item continues after the closing quote (`"a"b` is one item; the value
; parser rejects it later, 5.6).
; nu: lex.rs::lex_item

<interp-quoted-run> ::= "$" ( "\"" | "'" ) ... the same quote
; When the byte before the opening quote (within the item) is "$", an unescaped "("
; inside the string opens a subexpression; from there quotes and parens nest via
; interp_subexpr_step until the matching ")", and the string's own quote cannot
; close it. Backtick strings never interpolate. An unclosed "(" is Unclosed(")").
; nu: lex.rs::interp_subexpr_step, lex.rs::lex_item

<raw-string>      ::= "r" <hashes> "'" { <any byte> } "'" <hashes>
<hashes>          ::= 1*"#"
; triggered by "r#" at the top state of an item (also inside brackets); the same
; number of "#" must follow the closing "'"; a missing "'" after the hashes is
; Expected("'"); a missing closer is UnexpectedEof
; nu: lex.rs::lex_item, lex.rs::lex_raw_string

<bracketed-run>   ::= "[" <nested> "]" | "{" <nested> "}" | "(" <nested> ")"
                    | "<" <nested> ">"          ; only in signature mode
<nested>          ::= { <plain-byte> | <whitespace> | <eol> | <comment>
                      | <quoted-run> | <raw-string> | <bracketed-run> }
; Inside brackets whitespace, newlines, "|", ";" and special tokens are ordinary
; bytes. A closer must match the innermost opener: a mismatched "]" "}" ")" is
; Unbalanced, except that "]" with an EMPTY stack is a plain byte (`a]`); "}" or
; ")" with an empty stack is Unbalanced. In signature mode ">" pops an Angle frame
; and is otherwise a plain byte. An opener still open at the end is Unclosed.
; nu: lex.rs::lex_item
```

### 1.4 Re-lexing options per construct

Bracketed items are lexed again from their interior. Each construct passes its
own `additional_whitespace`, `special_tokens`, `skip_comment` and `in_signature`
to the lexer (`lex.rs::lex`, `lex.rs::lex_signature`):

| construct | additional whitespace | special tokens | comments | nu |
| --- | --- | --- | --- | --- |
| block, file | none | none | kept | parse_block |
| subexpression `( )`, `(...)` cell-path head | `\n\r` | none | skipped | parse_paren_expr, parse_full_cell_path |
| list `[ ]` | `\n\r,` | none | skipped | parse_list_expression |
| record `{ }` key | `\n\r,` | `:` | skipped | parse_record |
| record `{ }` value | `\n\r,` | none | skipped | parse_record |
| brace probe (first two tokens of `{ }`) | `\r\n\t` | `:` | skipped | parse_brace_expr |
| signature `[ ]` `( )` `\| \|` | `\n\r` | `:=,` in_signature | kept | parse_signature_helper |
| input/output types | `\n\r,` | none, in_signature | skipped | parse_input_output_types |
| generic type params `< >` | `\n\r` | `:,` | skipped | parse_type_params |
| cell path | `\n\r` | `.?!` | skipped | parse_cell_path |
| match block | ` \r\n,|` | none | skipped | parse_match_block_expression |
| list pattern | `\n\r,` | none | skipped | parse_list_pattern (then lite-parsed) |
| record pattern | `\n\r,` | `:` | skipped | parse_record_pattern |
| binary literal | `,\r\n` | none | skipped | parse_binary_with_base |
| assignment rhs | none | none | kept | parse_assignment_expression |
| `let x: <type>` items | none | `,` | skipped | parse_var_with_opt_type |

## 2. Blocks, pipelines and commands (the lite parse)

```ebnf
<block-tokens>    ::= { <separator> } [ <pipeline> { 1*<separator> <pipeline> } ] { <separator> }
; the lite parse of a <token-stream> (1.2)
<separator>       ::= <eol> | <semicolon> | <comment-line>
<comment-line>    ::= <comment> <eol>
; Comment lines directly above a command (no blank line between) are its leading
; comments (a def's documentation); a blank line clears them. A comment after a
; token on the same line is a trailing comment. Empty pipelines are dropped.
; nu: lite_parser.rs::lite_parse; parse_pipelines.rs::parse_block

<pipeline>        ::= <lite-pipeline> | <parsed-pipeline>
; the same text seen as tokens (this section) and after its items are parsed (2.1)
<lite-pipeline>   ::= [ <pipe> ] <command> { <pipe-continuation> <command> }
; A leading "|" yields an empty first command that is dropped (`( | str join)`).
; Two pipes in a row (`a | | b`) likewise drop the empty command.
; nu: lite_parser.rs::lite_parse, lite_parser.rs::LitePipeline::push

<pipe-continuation> ::= <pipe> [ <eol> ] { <comment-line> }
; After "|" the next command may start on a later line: the Eol directly after the
; pipe is ignored and ([Comment]+ Eol) pairs are skipped. ONLY ONE blank line is
; tolerated: at a second consecutive Eol the pipeline closes and the next command
; starts a new pipeline (`a |\n\n b` is two pipelines, no error). Symmetrically the
; lexer makes `\n|` a continuation but `\n\n|` keeps one Eol, so `a\n\n| b` is two
; pipelines, the second beginning with a dropped leading pipe. This changes meaning:
; `$x = 5\n\n| 3` leaves `$x = 5` in nu.
; nu: lite_parser.rs::last_non_comment_token, lite_parser.rs::lite_parse; lex.rs::lex_internal

; The trailing-pipe rule: a BLOCK whose last non-comment token is "|" (skipping ([Comment]+ Eol) pairs
; from the end) is UnexpectedEof("pipeline missing end"), whatever absorbed the
; pipe. So `ls |`, `ls | # c`, `ls |\n# c` (no final newline) and `alias x = ls |`
; are errors; `ls |\n`, `ls |\n\n`, `ls | # c\n` and `ls |\n# c\n` are not.
; nu: lite_parser.rs::lite_parse

<command>         ::= [ <attribute-lines> ] <command-body>
<command-body>    ::= <item> { <item> | <redirection> } [ <assignment-tail> ]
                    | <item> { <item> } <pipe-redirection>
; Items after a file redirection are still part of the command (`^echo x o> f extra`
; parses). A command with no items but a redirection is UnexpectedRedirection.
; nu: lite_parser.rs::lite_parse, lite_parser.rs::check_accepts_redirection, lite_parser.rs::try_add_redirection

<redirection>     ::= ( <out-redirect> | <err-redirect> | <out-err-redirect> ) <item>
; The target must be the very next token and must be an <item>; anything else
; (pipe, Eol, ";", another redirection, "=", "||", comment, end of input) is
; Expected("redirection target"). The target item is parsed as a value (shape Any).
; Combinations: the first redirection of any kind is Single; Stdout then Stderr or
; Stderr then Stdout is Separate; any other second, or any third, is
; MultipleRedirections.
; nu: lite_parser.rs::lite_parse, lite_parser.rs::try_add_redirection; parse_pipelines.rs::parse_redirection_target

<pipe-redirection> ::= <err-pipe> | <out-err-pipe>
; ends the command like "|" (it becomes the pipe of the next element) and records a
; Pipe target for stderr or both; same combination rules (`o> a e>| next` is
; Separate{out: File, err: Pipe})
; nu: lite_parser.rs::lite_parse

<assignment-tail> ::= <assignment-op> { <item> | <pipe> | <pipe-pipe> | <redirection-op> | <comment> }
; From the first <assignment-op> token to the end of the line everything, pipes and
; redirections included, belongs to the command; the line continues after a
; trailing "|" or before a leading "|" exactly as in <pipe-continuation> (one Eol).
; ";" ends the assignment. The tail is re-lexed and parsed as a block (3.3).
; nu: lite_parser.rs::lite_parse

<attribute-lines> ::= 1*<attribute-line>
<attribute-line>  ::= "@" <item-rest> { <item> | <pipe> | <pipe-pipe> | <redirection-op> | <assignment-op> } ( <eol> | <semicolon> )
; An item starting with "@" enters attribute mode ONLY when the previous token is
; Eol or ";" (i.e. at the start of a pipeline); `echo 1 | @foo` is a call to `@foo`.
; Inside an attribute line every token up to the Eol/";" is part of the attribute:
; pipes and redirections are NOT lite errors, they reach the attribute's argument
; parser as words. The definition must start on the very next line or after the
; ";": a blank line or a comment line between attributes and definition closes the
; command and the attributes then lack a definition (6.7).
; nu: lite_parser.rs::lite_parse
```

### 2.1 Which parser sees a command

```ebnf
<parsed-pipeline> ::= <single-command-pipeline> | <multi-command-pipeline>
; nu: parse_pipelines.rs::parse_pipeline

<single-command-pipeline> ::= <keyword-statement> | <pipeline-element>
; A pipeline with exactly one command goes through parse_builtin_commands: its first
; item (after any attribute lines) is matched against the keyword table:
;   def, extern, export, export-env, @attributes, let, const, mut, for, alias,
;   module, use, overlay, source, source-env, run, hide, where, plugin use
; Before the table, if the first item is not math-expression-like, not an
; unaliasable keyword and names an ALIAS, the alias is expanded.
; if/match/while/loop/try/return/break/continue are ordinary declared commands whose
; signatures use keyword and block shapes; nu parses them with parse_call.
; nu: parse_expressions.rs::parse_builtin_commands

<multi-command-pipeline> ::= <pipeline-element> { <pipeline-element> }
; Every element is parsed with parse_expression (3.1); the keyword table is NOT
; consulted. Elements after the first that mention `$in` are wrapped in `collect`
; (semantic).
; nu: parse_pipelines.rs::parse_pipeline_element
```

## 3. Expressions

### 3.1 Expression dispatch (`parse_expression`)

```ebnf
<pipeline-element> ::= { <env-shorthand> } <element-body>
; nu: parse_expressions.rs::parse_expression

<env-shorthand>   ::= <env-name> "=" [ <env-value> ]
<env-name>        ::= ( "_" | <ascii-letter> ) { "_" | <ascii-letter> | <ascii-digit> }
<env-value>       ::= "$" ...              ; parse_dollar_expr, shape Any
                    | <strict-string>      ; a fully single/double-quoted string
                                           ; (also $"..."), or a bare word
; The item is split at its FIRST "=". An item that fails these rules ends the
; shorthand prefix and starts the command. If every item is shorthand the element
; is UnknownCommand (`FOO=1` alone). Becomes `with-env {..} { <element-body> }`.
; nu: parse_expressions.rs::is_env_variable_name, parse_expressions.rs::parse_expression; parse_literals.rs::parse_string_strict

<element-body>    ::= <assignment>              ; if ANY remaining item is an <assignment-op>
                    | <math-expression>         ; if the first item is math-expression-like
                    | <statement-in-pipeline-error>
                    | <where-expr> | <run-expr>
                    | <call>
; The head classification applies to the first item after the shorthand:
;   def extern for module use source alias export export-env hide -> BuiltinCommandInPipeline
;   const mut                                    -> AssignInPipeline
;   overlay (unless the 2nd item is "list")      -> BuiltinCommandInPipeline
;   plugin (when the 2nd item is "use")          -> BuiltinCommandInPipeline
;   where                                        -> parse_where_expr (7.7)
;   run                                          -> parse_run_expr
;   anything else                                -> parse_call (4)
; parse_expression is used for (a) every element of a multi-command pipeline,
; (b) a single command whose head is not a keyword, and (c) the command after env
; shorthand, so the statement keywords are rejected in all three positions:
; `ls | hide ls`, `hide ls | length`, `FOO=1 def x [] {}`. `let` is NOT in the
; list: `ls | let x = 1` fails as an assignment (AssignmentRequiresVar) and
; `ls | let x` is a call to `let`.
; nu: parse_expressions.rs::parse_expression

; An item is math-expression-like when it is one of:
;   "true" | "false" | "null" | "not" | "if" | "match"
;   | "r#" ...
;   | ( "(" | "{" | "[" | "$" | "\"" | "'" | "-" ) ...
;   | <number> | <filesize> | <duration> | <datetime>
;   | <binary-literal>                 ; even one with invalid digits
;   | <range>                          ; whose bounds parse: a `$` bound names a
;                                      ; variable and a valid cell path on it, so
;                                      ; `..$`, `..$x.c!!` are external commands
; the literal probes are the section 5 parsers run speculatively
; nu: parse_expressions.rs::is_math_expression_like
```

### 3.2 Math expressions and operators

```ebnf
<math-expression> ::= <keyword-expression>
                    | <operand> { <operator> <operand-or-keyword> }
<keyword-expression> ::= <if> | <match>                 ; (6.6)
; a leading "if"/"match" hands ALL remaining items to parse_call (6.6); alone it
; is Expected("expression")
<operand>         ::= { "not" } <value>
; "not" may repeat; "not" with nothing after it is Expected("expression")
<operand-or-keyword> ::= <operand>
                    | <if> | <match>                   ; takes all remaining items
; An operator with no operand after it is IncompleteMathExpression.
; Folding: a stack that grows while precedence rises and collapses while it falls
; or stays level; "**" never collapses on equal precedence (right associative);
; every other operator is left associative.
; nu: parse_expressions.rs::parse_math_expression

<operator>        ::= "==" | "!=" | "<" | "<=" | ">" | ">="
                    | "=~" | "like" | "!~" | "not-like"
                    | "in" | "not-in" | "has" | "not-has"
                    | "starts-with" | "not-starts-with" | "ends-with" | "not-ends-with"
                    | "+" | "-" | "*" | "/" | "//" | "mod" | "**" | "++"
                    | "bit-or" | "bit-xor" | "bit-and" | "bit-shl" | "bit-shr"
                    | "and" | "or" | "xor"
; Recognised-but-rejected with a hint (UnknownOperator): "^" "pow" "is" "==="
; "contains" "%" "&" "<<" ">>" "bits-and" "bits-xor" "bits-or" "bits-shl"
; "bits-shr"; an <assignment-op> here is Expected("a non-assignment operator").
; nu: parse_expressions.rs::parse_operator
```

Precedence (nu-protocol `ast/operator.rs::Operator::precedence`; higher binds
tighter; `parse_math_expression` folds on it):

| prec | operators | assoc |
| --- | --- | --- |
| 100 | `**` | right |
| 95 | `*` `/` `mod` `//` | left |
| 90 | `+` `-` | left |
| 85 | `bit-shl` `bit-shr` | left |
| 80 | `==` `!=` `<` `<=` `>` `>=` `=~` `like` `!~` `not-like` `in` `not-in` `has` `not-has` `starts-with` `not-starts-with` `ends-with` `not-ends-with` `++` | left |
| 75 | `bit-and` | left |
| 70 | `bit-xor` | left |
| 60 | `bit-or` | left |
| 50 | `and` | left |
| 45 | `xor` | left |
| 40 | `or` | left |
| 10 | assignment operators (never inside a math expression) | n/a |

### 3.3 Assignment

```ebnf
<assignment>      ::= <assign-lhs> <assignment-op> <assign-rhs>
<assign-lhs>      ::= <full-cell-path>          ; re-parsed with parse_expression; must come out a FullCellPath
<assign-rhs>      ::= <block-tokens>            ; the absorbed tail (2), re-lexed and
                                                ; parsed as a block
; The operator is the FIRST assignment-op item. Empty lhs / rhs are errors.
; lhs: the parsed lhs must be a FullCellPath; its head must be a mutable Var or
; `$env` (semantic). Any FullCellPath is accepted syntactically: `(1) = 2`,
; `[1].0 = 2`, `{a: 1}.a = 2`; a non-FullCellPath (`1 = 2`, `[1] = 2`,
; `{a: 1} = 2`, `echo a = b`) is AssignmentRequiresVar.
; rhs: an unknown (external) command at the start of the rhs must be written with
; "^" ("External command calls must be explicit in assignments").
; nu: parse_expressions.rs::parse_assignment_expression
```

## 4. Calls

```ebnf
<call>            ::= <external-call> | <sigil-call> | <keyword-command> | <internal-call>
; nu: parse_calls.rs::parse_call

<keyword-command> ::= <if> | <match> | <while> | <loop> | <try> | <return> | <break> | <continue>
; ordinary declared commands whose signatures use keyword and block shapes (6.6),
; parsed with parse_call like any other command
; nu: nu-cmd-lang core_commands signatures; parse_calls.rs::parse_internal_call

<external-call>   ::= "^" <ext-head> { <ext-arg> }
                    | <unknown-head> { <ext-arg> }  ; no "^": a head that names no command
<unknown-head>    ::= <ext-string>
; a bare head that names no declaration (a built-in, keyword or prelude command of the
; table, or a def/extern/alias in scope: <call-head> matched none), or that
; names an alias of an external call: parse_call tries a leading ".." range
; and then calls parse_external_call, so every argument is an <ext-arg> and
; `git log 0b2d1f4..HEAD` is three external strings.
; nu: parse_calls.rs::parse_call (the fall-through after find_longest_decl, and
;     an alias whose wrapped call is an ExternalCall)
<ext-head>        ::= <var-item> | <paren-item>       ; parsed as an expression
                    | <ext-string>
                    | <the empty string>              ; `^` alone parses, fails at run time
<ext-arg>         ::= "..." ( <bracket-item> | <var-item> | <paren-item> )   ; spread
                    | <var-item> | <paren-item> | <bracket-item> | <brace-item>
                    | <ext-string>
; a "[" argument goes to parse_list_expression alone, which needs the item to END
; with "]": `^cmd [a].x`, `^cmd [a].0`, `^cmd ...[a].x` are Unclosed("]"), while
; `^cmd (ls).name` and `^cmd {a: 1}.a` parse. The same holds after an <unknown-head>
; (`se [a].0`).
; nu: parse_calls.rs::parse_external_call, parse_calls.rs::parse_external_arg, parse_calls.rs::parse_regular_external_arg;
;     parse_expressions.rs::parse_list_expression

<ext-string>      ::= <raw-string>
                    | <bare-glob>                     ; no quote, paren or backtick in it
                    | 1*<segment>
<segment>         ::= <bare-text> | "\"" ... "\"" | "'" ... "'" | "$" <quoted-run>
                    | "`" ... "`" | "(" ... ")"
; Segments are each parsed as a string (5.6) and concatenated: all-literal segments
; give one string (a GlobPattern unless the whole word is quoted), otherwise a
; (Glob)Interpolation. So `--query='a (b)'` keeps its parens literal while
; `--out=(pwd)/x` interpolates. "\" escapes only inside "...".
; nu: parse_calls.rs::parse_external_string

<sigil-call>      ::= "%" <builtin-head> <args>              ; `%ls`: forced built-in
                    | "%" " " <builtin-head> <args>          ; `% ls` (two items)
                    | "%" ( <var-item> | <paren-item> ) <args>   ; dynamic head
; "%" with a quoted, list, record, "^" or "%" head is "percent sigil requires a
; built-in command"; a name that is not a built-in is the same error.
; nu: parse_calls.rs::parse_call

<internal-call>   ::= <call-head> <args>
<call-head>       ::= <word> { " " <word> }
; the LONGEST run of leading words naming a known declaration (`str trim`,
; `overlay use`, `attr example`); an alias head may be followed by subcommand
; words of the aliased command. A head naming no declaration is an <unknown-head>. A quoted word is never a head (`"ls" -l` is a
; math-expression error) because is_math_expression_like fires on the quote.
; nu: parse_calls.rs::find_longest_decl_with_prefix

<args>            ::= { <arg> }
<arg>             ::= <long-flag> | <short-flags> | "--" | <spread-arg> | <keyword-arg> | <positional>
; nu: parse_calls.rs::parse_internal_call

<long-flag>       ::= "--" <flag-name> [ "=" <value> ]
                    | "--" <flag-name> <value>      ; when the signature says the flag takes a value
; `--flag=value` on a switch parses value as Boolean; "--" exactly is end of
; options: everything after is positional. Unknown flags are UnknownFlag.
; nu: parse_calls.rs::parse_long_flag

<short-flags>     ::= "-" 1*<char>
; each char must be a known short flag; only the LAST may take a value. `-1` /
; `-1.5` / `-.5` is a negative number instead when the current positional's shape
; is Int/Number/Float (or a OneOf containing one); for any other shape it is
; "doesn't have flag -1" (so `echo -1` and `return -1` are errors in nu).
; nu: parse_calls.rs::parse_short_flags, parse_calls.rs::shape_allows_negative_number

<spread-arg>      ::= "..." ( <bracket-item> | <brace-item> | <var-item> | <paren-item> )
; a list spread needs a rest param or --wrapped; a record spread needs at least one
; named flag; a list spread may not skip a required positional (all semantic)
; nu: parse_calls.rs::parse_internal_call; parse_helpers.rs::extract_spread_list,
;     parse_helpers.rs::extract_spread_record

<positional>      ::= <value-of-shape>
; parsed with the SyntaxShape of the next positional in the signature; multi-span
; shapes (MathExpression, RowCondition, Expression, Signature, ExternalSignature,
; Keyword, VarWithOptType, OneOf of those) consume several items; where they end is
; computed from keyword positions and remaining required positionals.
; nu: parse_calls.rs::parse_multispan_value, parse_calls.rs::calculate_end_span, parse_calls.rs::parse_oneof

<keyword-arg>     ::= <keyword-word> <value-of-shape>
; SyntaxShape::Keyword(kw, shape): the item must equal kw exactly, then the inner
; shape follows; a missing inner value is KeywordMissingArgument
; nu: parse_calls.rs::parse_multispan_value
```

## 5. Values and literals

Each rule describes the text of one item.

### 5.1 Dispatch

```ebnf
<value>           ::= <dollar-expr>            ; item starts with "$"      (5.9)
                    | <paren-expr>             ; item starts with "("      (5.12)
                    | <brace-expr>             ; item starts with "{"      (5.13)
                    | <bracket-expr>           ; item starts with "["      (5.14)
                    | <raw-string>             ; item starts with "r#"     (5.6)
                    | <shaped-value>           ; anything else, by expected shape
; nu: parse_expressions.rs::parse_value

<bracket-expr>    ::= <list-or-table> [ <cell-path-tail> ]
; only when the expected shape is Any, List, Table, Signature, ExternalSignature,
; Filepath, String, GlobPattern or ExternalArgument; every other shape rejects "["
; nu: parse_expressions.rs::parse_value

<shaped-value>    ::= <any-value>              ; shape Any
                    | <int>                    ; Int
                    | <float>                  ; Float
                    | <int> | <float>          ; Number (int first)
                    | <duration> | <filesize> | <datetime> | <range> | <binary>
                    | <nothing>                ; Nothing
                    | <bool>                   ; Boolean
                    | <string>                 ; String, Filepath, Directory, GlobPattern:
                                               ; but NOT "true", "false" or "null"
                    | <cell-path-literal-body>  ; CellPath (members without "$.")
                    | <external-arg>           ; ExternalArgument (4)
; Block, Closure and Record shapes are satisfied only by a "{" item.
; nu: parse_expressions.rs::parse_value

<any-value>       ::= <nothing> | <bool>
                    | <binary>       ; 1st: only "0x["/"0o["/"0b[" prefixes; bad digit = hard error
                    | <range>        ; 2nd: fails softly when not range-shaped
                    | <filesize>     ; 3rd: unit suffix with a bad number = hard error
                    | <duration>     ; 4th: same
                    | <datetime>     ; 5th
                    | <int>          ; 6th: "0x"/"0o"/"0b" prefix with bad digits = hard error
                    | <float>        ; 7th
                    | <string>       ; 8th: any remaining text
; "hard error" = not an `Expected` error, so the item is garbage, never a string
; nu: parse_expressions.rs::parse_value
```

### 5.2 Booleans, nothing, numbers

```ebnf
<bool>            ::= "true" | "false"
<nothing>         ::= "null"
; nu: parse_expressions.rs::parse_value

<int>             ::= <int-text>                 ; after deleting every "_"
<int-text>        ::= "0b" [ "+" ] 1*<digit-2> | "0o" [ "+" ] 1*<digit-8>
                    | "0x" [ "+" ] 1*<digit-16>
                    | [ "+" | "-" ] 1*<digit-10>
; radix literals are parsed as u64 and reinterpreted (0xffffffffffffffff = -1); a
; "+" after a radix prefix is accepted ("0x+ff" = 255, as u64::from_str_radix takes
; it), a "-" is not; decimal must fit in i64 ("+7" is fine). "_" may appear
; anywhere ("_1", "1_", "0x_f"). A radix prefix followed by anything that is not all
; digits of that radix is the hard error "invalid digits for radix N".
; nu: parse_literals.rs::parse_int, parse_literals.rs::strip_underscores

<float>           ::= <float-text>               ; after deleting every "_"
<float-text>      ::= [ "+" | "-" ] ( 1*<digit> [ "." { <digit> } ] | "." 1*<digit> )
                      [ ( "e" | "E" ) [ "+" | "-" ] 1*<digit> ]
                    | [ "+" | "-" ] ( "inf" | "infinity" | "nan" )    ; case-insensitive
; exactly what Rust's f64::from_str accepts
; nu: parse_literals.rs::parse_float, parse_literals.rs::parse_number
```

### 5.3 Filesize and duration

```ebnf
; a unit value is <unit-number> followed by a filesize or duration unit:
<unit-number>     ::= ( <digit> | "." <digit> | "-" <digit> ) <any-chars>
; the first two bytes gate the attempt; the text before the unit, minus "_", must
; then parse as f64 ("1e3kb", ".5kb", "-3sec", "1_000kb" are fine; "1..2sec" is the
; hard error "value must be a number"); a number ending in "$" is never a unit value
<filesize>        ::= <unit-number> <filesize-unit>     ; never when the item starts with "0x"
<filesize-unit>   ::= "KB" | "MB" | "GB" | "TB" | "PB" | "EB"
                    | "KIB" | "MIB" | "GIB" | "TIB" | "PIB" | "EIB" | "B"
; matched against the ASCII-uppercased item (case-insensitive), in this order
<duration>        ::= <unit-number> <duration-unit>
<duration-unit>   ::= "ns" | "us" | "µs" | "μs" | "ms" | "sec" | "min" | "hr" | "day" | "wk"
; U+00B5 and U+03BC both spell microseconds; units are case-sensitive
; nu: parse_literals.rs::parse_unit_value, parse_literals.rs::parse_duration,
;     parse_literals.rs::parse_filesize
```

### 5.4 Datetime

```ebnf
<datetime>        ::= <date>                              ; "T00:00:00+00:00" appended
                    | <date> <time-sep> <time>            ; "+00:00" appended
                    | <date> <time-sep> <time> <offset>   ; full RFC 3339
<date>            ::= 4*<digit> "-" 2*<digit> "-" 2*<digit>   ; item >= 6 bytes, byte 4 is "-"
<time-sep>        ::= "T" | "t"
<time>            ::= 2*<digit> ":" 2*<digit> ":" 2*<digit> [ "." 1*<digit> ]
<offset>          ::= "Z" | "z" | ( "+" | "-" ) 2*<digit> ":" 2*<digit>
; Validity is chrono's parse_from_rfc3339: month 1-12, day valid for that month and
; year (2023-02-30 is a string), hour < 24, minute < 60, second <= 60, offset
; hours < 24; seconds are mandatory; "+0530" (no colon) is not an offset. Anything
; that fails is a plain string.
; nu: parse_literals.rs::parse_datetime
```

### 5.5 Binary

```ebnf
<binary>          ::= <binary-prefix> { <binary-sep> } { 1*<radix-digit> { <binary-sep> } } "]"
<binary-prefix>   ::= "0x[" | "0o[" | "0b["
<binary-sep>      ::= <whitespace> | "," | <eol> | ";" | <comment>
; digit runs are concatenated, left-padded with "0" to a multiple of 2 (hex), 3
; (octal) or 8 (binary) digits, and split into bytes; a chunk that is not a byte
; ("0o[777]", "0x[zz]") is InvalidBinaryString. A "|", redirection or assignment
; token inside is "expected binary".
; nu: parse_literals.rs::parse_binary, parse_literals.rs::parse_binary_with_base, parse_literals.rs::decode_with_base
```

### 5.6 Strings

```ebnf
<string>          ::= <bare-interpolation>     ; no leading quote and contains "("  (5.7)
                    | <double-quoted> | <single-quoted> | <backtick-quoted>
                    | <raw-string>
                    | <bare-word>
; the empty item is "expected String"
; nu: parse_literals.rs::parse_string, parse_literals.rs::is_bare_string_interpolation

<bare-word>       ::= 1*<bare-char>
; any byte the lexer lets into an item (1.3) in the current context. Quotes inside a
; bare word are kept literally ("a"b"c" is a"b"c). The value is the text unchanged.
; nu: parse_literals.rs::unescape_unquote_string; parse_helpers.rs::trim_quotes

<double-quoted>   ::= "\"" { <dq-char> | <escape> } "\""
; the LAST "\"" of the item must be its final byte ("a"b is ExtraTokens); earlier
; quotes are literal
<single-quoted>   ::= "'" { <byte except "'"> } "'"      ; no escapes; same last-quote rule
<backtick-quoted> ::= "`" { <byte except "`"> } "`"      ; no escapes; NOT checked for
                                                          ; trailing text: `a`b is the
                                                          ; literal string `a`b
; nu: parse_literals.rs::check_string_no_trailing_tokens (only the double and the single quote)

<escape>          ::= "\" ( "\"" | "'" | "\" | "/" | "(" | ")" | "{" | "}" | "$" | "^"
                          | "#" | "|" | "~" | " " | "a" | "b" | "e" | "f" | "n" | "r" | "t" | "0" )
                    | "\x" <hex> <hex>            ; any byte 00-FF; the whole string must
                                                  ; then be valid UTF-8 ("\xC3\xA9" is "é")
                    | "\u{" 1*6<hex> "}"          ; a valid scalar (no surrogates, <= 10FFFF)
; \a=07 \b=08 \e=1b \f=0c; any other "\X" or a trailing "\" is an error
; nu: parse_literals.rs::unescape_string, parse_literals.rs::parse_hex_escape, parse_literals.rs::parse_unicode_escape

; <raw-string> and <hashes> are defined with the lexer (1.3): the same "#" count on
; both sides; the body may contain "'" followed by fewer "#"
; nu: parse_literals.rs::parse_raw_string
```

### 5.7 Interpolation

```ebnf
<interpolation>   ::= "$" "\"" { <dq-text> | <interp-expr> } "\""   ; text parts unescaped
                    | "$" "'"  { <sq-text> | <interp-expr> } "'"    ; no escapes
                    | <bare-interpolation>
<bare-interpolation> ::= { <bare-text> | <interp-expr> }             ; the whole item
<interp-expr>     ::= "(" <block-tokens> ")"
; In $"..." a "(" preceded by an ODD number of backslashes is text ("\("); otherwise
; it opens an expression; in $'...' and bare words every "(" opens one. Inside the
; expression quotes and nested parens are tracked as in the lexer, so ")" inside a
; quoted string does not close it. The "( ... )" text is parsed as a subexpression
; head (5.12): it may span lines and hold a whole pipeline; a cell-path tail cannot
; follow it (".a" after ")" is text). The last-quote rule applies to the closer.
; nu: parse_literals.rs::parse_string_interpolation, parse_literals.rs::is_bare_string_interpolation
```

For the Filepath, Directory and GlobPattern shapes the same text yields
`Filepath` / `Directory` / `GlobPattern` (with a `quoted` flag) or, for a bare
interpolation, `StringInterpolation` / `GlobInterpolation`. That distinction is
signature driven (`parse_literals.rs::parse_path_like`).

### 5.8 Variables

```ebnf
<variable>        ::= "$" <var-name>
<var-name>        ::= 1*<identifier-byte>
<identifier-byte> ::= any byte except "." "[" "(" "{" "+" "-" "*" "^" "%" "/" "=" "!" "<" ">" "&" "|"
; so "$a-b" is invalid, "$a_b", "$été" and "$x?" are names (but "?" is a cell-path
; special byte and never reaches this rule). A bare "$" is "Incomplete variable".
; Reserved names with fixed ids: "$nu", "$in", "$env", "$ans". "$it" is an ordinary
; name bound implicitly by row conditions (7.7). Existence is semantic.
; nu: parse_literals.rs::parse_variable_expr; parse_helpers.rs::is_identifier_byte
```

### 5.9 `$` items

```ebnf
<dollar-expr>     ::= <interpolation>                    ; "$\"" or "$'" prefix
                    | "$." <cell-path-literal-body>      ; cell-path literal; "$." alone is the empty path
                    | <range>                            ; if range-shaped (5.10)
                    | <full-cell-path>                   ; otherwise (5.11)
; nu: parse_literals.rs::parse_dollar_expr
```

### 5.10 Ranges

```ebnf
<range>           ::= [ <bound> ] [ ".." <bound> ] <range-op> [ <bound> ]
                    ; not both first and last bound absent (".." alone is `cd ..`)
<range-op>        ::= ".." | "..<" | "..="
<bound>           ::= <int> | <float>                              ; shape Number
                    | <variable> [ <cell-path-tail> ]              ; "$x", "$x.a"
                    | "(" <block-tokens> ")" [ <cell-path-tail> ]  ; "(1 + 1)", "(ls).0"
; a "{", "[" or bare-word bound fails, and the whole item falls through to the next
; shape (it becomes a string). On the text of the item, before parsing bounds:
; an item starting with "..." is never a range (spread); the ".." occurrences
; counted are those at parenthesis depth 0 (quotes are not considered); exactly one
; or two must exist; with two, the first is the "next" operator; "..<" may only be
; the range operator; "..=" is recognised only at the range-operator position;
; "1...5" is the range 1 .. 0.5.
; nu: parse_literals.rs::parse_range
```

### 5.11 Cell paths

```ebnf
<cell-path-literal-body> ::= [ <member> { <member-sep> <member> } ]     ; after "$."
<full-cell-path>  ::= <cell-path-head> <cell-path-tail>
<cell-path-head>  ::= <variable>
                    | "(" <block-tokens> ")"
                    | "[" <list-or-table-body> "]"
                    | "{" <record-body> "}"
                    | <bare-word>          ; ONLY with an implicit head (row conditions):
                                           ; the bare word is the first member, no "."
<cell-path-tail>  ::= { <member-sep> <member> } [ "." ]                 ; trailing "." accepted
<member-sep>      ::= "." | "?" "." | "!" "." | "?" "!" "." | "!" "?" "."
; "?" makes the PRECEDING member optional, "!" case-insensitive; at most one of
; each, either order; they may also end the path ("$x.a?", "$x.a!?")
<member>          ::= <int>                ; non-negative after "_" removal; "-1" is
                                           ; "negative index is not supported"
                    | <string>             ; bare word, "...", '...', `...`; a member
                                           ; containing "(" (bare interpolation) is an error
; The item is lexed with "." "?" "!" special, so "a.b" splits but "a\"b.c\"" does not;
; newlines inside are whitespace. A "(...)" head is parsed as a block (a whole
; pipeline list, so `(ls | length).0` works).
; nu: parse_literals.rs::parse_cell_path, parse_literals.rs::parse_simple_cell_path, parse_literals.rs::parse_full_cell_path
```

### 5.12 Parenthesised items

```ebnf
<paren-expr>      ::= <range>                                  ; when range-shaped: "(1)..(3)"
                    | "(" <params> ")"                         ; shape Signature only (7.1)
                    | "(" <block-tokens> ")" <cell-path-tail>  ; subexpression; newlines
                                                               ; are whitespace inside
                    | <bare-interpolation>                     ; when the "(" group closes
                                                               ; before the item ends: "(pwd)/x"
; nu: parse_literals.rs::parse_paren_expr
```

### 5.13 Brace items: record, closure, block

```ebnf
<brace-expr>      ::= <record> | <closure> | <block> | <match-block> | <full-cell-path>
; Decided from the expected shape and the first two tokens of the body (1.4 brace probe):
;   body empty           : Closure -> closure; Block -> block; MatchBlock -> match block;
;                          any other shape -> empty record
;   first token "|"/"||" : Block -> "expected block, found closure"; else closure
;   second token ":"     : record (parsed as a full cell path: "{a: 1}.a" works)
;   otherwise            : Closure -> closure; Block -> block; MatchBlock -> match block;
;                          item not ending in "}" -> full cell path ("{}.foo?");
;                          first token "...$x"/"...{"/"...(" -> record (spread first);
;                          Any -> closure; any other shape -> "non-block value"
; So a record where a Block is required (`if true {a: 1}`) is a type mismatch.
; nu: parse_literals.rs::parse_brace_expr

<closure>         ::= "{" [ <closure-params> ] <block-tokens> "}"
<closure-params>  ::= "|" <params> "|" | "||"      ; may be preceded by newlines (7.1)
; nu: parse_expressions.rs::parse_closure_expression

<block>           ::= "{" <block-tokens> "}"       ; a leading "|" is "expected block but found closure"
; where a keyword wants a <block> or a closure (if, while, loop, for, try, catch,
; finally) nu also takes a <variable>, a subexpression, and a "{" item whose second
; token (probed as in 5.13, a tail such as `.a` included) is ":": a record or a
; cell path on one. nu types it any, which passes, unless it is a record whose keys
; are all plain strings (not raw), which is a type error: `if true { $env.A:b }` and
; `try {a: 1}.a` parse, `if true {a: 1}` does not
; nu: parse_expressions.rs::parse_block_expression; parse_literals.rs::parse_brace_expr;
;     parse_expressions.rs::parse_record (its type)
```

### 5.14 Lists and tables

```ebnf
<list-or-table>   ::= <table> | <list>
<list>            ::= "[" { <list-sep> } { <list-item> { <list-sep> } } "]"
<list-sep>        ::= <whitespace> | "," | <eol> | "|" | <comment>
; "|" splits the body into pipeline elements whose items are all list items; a
; trailing "|" is "Unexpected end of code"; "||" is ShellOrOr; ";" is "Unexpected
; semicolon in list"; a redirection token (`o> file`) is consumed by the lite parser
; and silently dropped ([a o> b] is ["a"]; nothing before it, no target, or a second
; one for the same stream is an error); an assignment-op item ("=") is a bare word
; and, as in every lite parse, everything after it is one (`[a = b | c]` has five).
<list-item>       ::= <value> | <spread>
<spread>          ::= "..." ( "[" <list-body> "]" | <variable> [ <cell-path-tail> ] | "(" <block-tokens> ")" )
; "...x" with any other first byte is the bare word "...x"
; nu: parse_expressions.rs::parse_list_expression; parse_helpers.rs::extract_spread_value

<table>           ::= "[" <list> ";" <list> { <list-sep> <list> } "]"
; The item is a table iff its body's first token starts with "[" and its second is
; ";". Then: no row after ";" is "expected table row"; every row token must start
; with "[" (a second ";" is an error); no spreads in header or rows; each row must
; have exactly as many items as the header (MissingColumns / ExtraColumns); every
; header item must be a string or it is "Table column name not string".
; nu: parse_expressions.rs::parse_table_expression, parse_expressions.rs::parse_table_row, parse_expressions.rs::table_type
```

### 5.15 Records

```ebnf
<record>          ::= "{" { <record-sep> } { <record-entry> { <record-sep> } } "}"
<record-sep>      ::= <whitespace> | "," | <eol> | <comment>
<record-entry>    ::= <record-key> ":" <record-value>
                    | <record-spread>
<record-key>      ::= <value> with shape String, lexed with ":" special (so "a:1" splits):
                      <bare-word> (no ":"; not "true"/"false"/"null"; a "(" makes it an
                      interpolation whose literal parts may not contain ":"),
                      <double-quoted> | <single-quoted> | <backtick-quoted> | <raw-string>,
                      <variable> [ <cell-path-tail> ], "(" <block-tokens> ")" [ <cell-path-tail> ],
                      "[" ... "]" [ <cell-path-tail> ], "$\"...\""
                      ; a "{" key is "non-block value" unless it is itself "{k: v}"
<record-value>    ::= <value> with shape Any, lexed with NO special bytes ("http://x" is
                      one item), but a bare word or bare interpolation containing ":" is
                      "colon in bare word specifying record value"
<record-spread>   ::= "..." ( "{" <record-body> "}" | <variable> [ <cell-path-tail> ] | "(" <block-tokens> ")" )
; Key, ":" and value are lexed one token at a time in that order; the ":" must be
; its own token ("a:1", "a: 1", "a : 1" all work). A key, ":" or value token that is
; not an item ("|", "||", ";", "=", "+=", "o>") is "Unexpected token in record";
; a key with no ":" is "Incomplete record field"; ":" then nothing is an error.
; nu: parse_expressions.rs::parse_record, parse_expressions.rs::check_record_key_or_value
```

## 6. Statements

Vocabulary: `<expression>` is a `<pipeline-element>` (3.1); `<block>` is a
`{ ... }` block without parameters (5.13); `<closure>` is a closure literal;
`<string>` is 5.6; `<var-decl>` is 7.4; `<params>` and `<io-types>` are 7.1-7.3;
`<pattern>` is 7.6.

### 6.1 Keyword table and pipeline restrictions

```ebnf
<keyword-statement> ::= <def> | <extern> | <export> | <export-env> | <attribute-block>
                    | <let> | <const> | <mut> | <for> | <alias> | <module> | <use>
                    | <overlay-stmt> | <source> | <source-env> | <run> | <hide>
                    | <where> | <plugin-use>
; only for a single-command pipeline (2.1)
; nu: parse_expressions.rs::parse_builtin_commands

; Every keyword is a command with a fixed signature, so at the start of each of its
; positional arguments nu looks for flags (any number of them): "--help"/"-h"
; there makes the whole statement an ordinary call (`def foo [] --help`, `for x
; --help in [] {}`, `alias alias --help`), the first "--" is consumed and dropped,
; and any other "-x" is "doesn't have flag" (`return -1`, `match -1 {}`, `if -1 >
; 0 {}`, `where -1 > 0`). Where a positional spans several items (a condition, a
; signature, an alias target) only its first item is such a boundary.
; After "--help" nu goes on parsing the positionals that follow and only forgives
; the MISSING ones: `match 1 --help :{}` has no match block, `return --help 1 2`
; and `loop --help {} y` have an extra positional, `def foo [] {} {} --help` has no
; colon, but `for x --help in []` still lacks the argument of `in` (6.6). After "--" no
; flag is looked for at all: a second "--" is a positional (`try {} -- catch {}
; --` has one too many), `return -- --help` returns a string.
; The statements nu parses by position never see "--": `alias -- x = ls`, `alias x
; -- = ls`, `module -- x {}`, `module x -- {}`, `let -- x = 1`, `export-env -- {}`
; are errors ("missing sign", "expected block", "not a valid variable name"). An
; alias help call must be the whole statement: `alias --help x = ls` and `alias x
; --help extra` are "missing sign". `export --help x` has an extra positional.
; nu: parse_calls.rs::parse_internal_call (parse_long_flag, parse_short_flags per positional,
;     `end_of_options`), parse_calls.rs::check_call; parse_alias.rs::check_alias_name

<pipeline-forbidden-head> ::= "def" | "extern" | "for" | "module" | "use" | "source"
                    | "alias" | "export" | "export-env" | "hide" | "const" | "mut"
                    | "overlay" (unless followed by "list") | "plugin" "use"
; refused with "statement used in pipeline" at ANY position of a multi-element
; pipeline, including the first, and after env shorthand (3.1). `let` is not in
; the list, but `ls | let x = 1` fails because the "=" routes it to assignment.
; nu: parse_expressions.rs::parse_expression

; Redirections are refused on: def, extern, alias, use, module, for, source,
; source-env, run, hide, overlay* (by the word "overlay" alone, before its
; arguments), plugin use, export*. A redirection on these is
; RedirectingBuiltinCommand. On export-env it is dropped without a look. `let x =
; ls o> f` is not an error because the redirection is absorbed into the rhs pipeline.
; nu: parse_pipelines.rs::redirecting_builtin_error and its call sites in parse_def.rs,
;     parse_alias.rs, parse_module.rs, parse_source.rs and
;     parse_expressions.rs::parse_builtin_commands

<parser-keyword>  ::= "if" | "match" | "try" | "overlay" | "overlay hide"
                    | "overlay new" | "overlay use"                    ; aliasable
                    | "alias" | "const" | "def" | "extern" | "module" | "use" | "export"
                    | "export alias" | "export const" | "export def" | "export extern"
                    | "export module" | "export use" | "for" | "loop" | "while"
                    | "return" | "break" | "continue" | "let" | "mut" | "hide"
                    | "export-env" | "source-env" | "source" | "run" | "where"
                    | "plugin use"                                     ; unaliasable
; a def/extern/alias NAME equal to any entry (after quote removal) is NameIsKeyword;
; the multi-word entries match quoted names (`def "export def"`)
; nu: parse_keywords.rs::ALIASABLE_PARSER_KEYWORDS, parse_keywords.rs::UNALIASABLE_PARSER_KEYWORDS, parse_keywords.rs::reject_parser_keyword_name

<bad-definition-name> ::= a name containing "#" | "^" | "%"
                    | a name parsing as a filesize ("1kb") or f64 ("5", "1.5", "inf")
; nu: parse_def.rs::parse_def_predecl, parse_alias.rs::parse_alias

; Predeclaration: before a block is parsed, every single-command pipeline of the form
; [ "export" ] ( "def" | "extern" ) { "--flag" } <name> ... "[" | "(" declares <name>,
; so calls to commands defined later in the file resolve; the same name declared
; twice this way in one block is DuplicateCommandDef (an alias is not predeclared,
; so `def foo` plus `alias foo` is fine, and so is a nested block)
; nu: parse_def.rs::parse_def_predecl
```

### 6.2 let, mut, const

```ebnf
<let>             ::= "let"   <var-decl> [ "=" <rhs-pipeline> ]
<mut>             ::= "mut"   <var-decl>   "=" <rhs-pipeline>
<const>           ::= "const" <var-decl>   "=" <rhs-pipeline>
<rhs-pipeline>    ::= <assignment-tail> body parsed as <block-tokens>   ; (2) everything to
                                                                        ; the end of the line
; `let x` without a value is allowed (initial_value is optional in let's signature);
; `mut x` and `const x` are MissingPositional. A redirection or "|" in the rhs is
; part of the rhs pipeline. Type checks and const-evaluability: semantic.
; nu: parse_bindings.rs::parse_let, parse_bindings.rs::parse_mut, parse_bindings.rs::parse_const;
;     nu-cmd-lang core_commands/{let_,mut_,const_}.rs signatures
```

### 6.3 def, extern

```ebnf
<def>             ::= "def" { <def-flag> } <def-name> { <def-flag> } <full-signature> <def-body>
<def-flag>        ::= "--env" | "--wrapped"
; the flags are ordinary named arguments, so they parse anywhere in principle, but a
; flag between the signature and the body breaks the multi-span signature and one
; after the body leaves the body missing: in practice only before/after the name.
; nu: parse_def.rs::parse_def (via parse_internal_call)

<def-name>        ::= <string> - <reserved-definition-name>   ; bare or quoted (multi-word)
<reserved-definition-name> ::= <parser-keyword> | <bad-definition-name>
; must be a string item: `def $x` and `def $"x"` are "expected string"; a name with
; "(" or "[" anywhere in its text, quoted or not (`def (foo)`, `def [foo]`,
; `def "a(b"`, `def foo[]`), is "no space between name and parameters"
; nu: parse_def.rs::detect_params_in_name; SyntaxShape::String

<full-signature>  ::= <signature-item>
                    | <signature-item> <brace-item>         ; the brace item is DROPPED (never parsed):
                                                            ; `def f [] {} {}`, `extern f [] {}`
                    | <signature-item> ":" <io-types>       ; "[]" ":" "int -> string"
                    | <signature-item> ":"                  ; colon and nothing after it
                    | <signature-item-colon> <io-types>     ; "[]:" glued colon
<signature-item>  ::= "[" <params> "]" | "(" <params> ")"
<signature-item-colon> ::= <signature-item> ":"
; The signature argument of def/extern receives every item up to (def) or including
; (extern) the last one: one item is the signature; two of which the second starts
; with "{" is the signature and an item nu drops on the floor; otherwise the io
; types are the items after a ":" (glued or alone), concatenated and re-lexed with
; "," as whitespace (7.3), possibly none; items there without a ":" are the error
; "colon (:) before type signature". `[]:` with nothing after it is Unclosed.
; nu: parse_signatures.rs::parse_full_signature, parse_signatures.rs::parse_signature;
;     parse_calls.rs::calculate_end_span

<def-body>        ::= <closure>
; def and export def both go through the `def` declaration (SyntaxShape::Closure):
; the body is parsed as a closure before its shape is looked at, so `{|x| }` parses
; (the parameters are then replaced by the signature) and `{a: 1}` is code (a call
; to `a:`). Because the signature takes every item but the last, `def foo [] {} extra`
; fails on `extra` as the body while `def foo [] {} {}` drops the first `{}`.
; `--wrapped` needs a `...rest` param that is untyped or typed `string`. An untyped
; one gets the shape external_arg, found on the text (a `:` after `...name`), so
; a call to the command takes <ext-arg>s: `f 'x'$` and `f 0b2` parse, from the
; predeclaration on and through an alias of the command. The positionals declared
; before the rest keep their own shapes; every argument past them, and any flag
; the signature does not know, is an <ext-arg> (allows_unknown_args).
; nu: parse_calls.rs::parse_internal_call, parse_calls.rs::parse_unknown_arg;
;     parse_def.rs::rest_param_is_type_annotated, parse_def.rs::parse_def_predecl,
;     parse_def.rs::parse_def_inner; nu-cmd-lang core_commands/def.rs

<extern>          ::= "extern" <def-name> <full-signature>
; The signature argument takes every remaining item and the signature declares no
; body, so nothing reaches the body slot of parse_extern_inner (dead code):
; `extern foo [] {}` drops the `{}` unparsed and `extern foo [] {} {}` is
; "colon (:) before type signature". No flags. For extern signatures a
; default-value token is not parsed at all (`extern foo [x = 0x]` parses; the
; token is ignored text).
; nu: parse_def.rs::parse_extern, parse_def.rs::parse_extern_inner; parse_calls.rs::calculate_end_span
```

### 6.4 alias

```ebnf
<alias>           ::= "alias" <def-name> "=" <alias-target>
                    | "export" "alias" <def-name> "="             ; no target: accepted (see below)
; `alias x=y`, `alias x`, `alias x =`, `alias = x` are errors; `export alias x =` is
; NOT, because the "incomplete alias" check counts items without the `export` word
<alias-target>    ::= <call> over ALL remaining items, pipes and redirections included,
                      as bare words (assignment mode)
; refused: a target whose first item is math-expression-like (`1 + 1`, `$x`, `(ls)`,
; `[1]`, `{ }`, `not ...`, `"ls"`, `-1`) => CantAliasExpression, except `if` and
; `match`; a target that is an unaliasable keyword (`alias d = def`, `alias l = let`,
; `alias e = export def`) => CantAliasKeyword; aliasable keywords and any external
; call (`^ls -l`, and `FOO=1 ls`, which parses as the external `FOO=1`) are fine.
; nu: parse_alias.rs::parse_alias
```

### 6.5 Modules

```ebnf
<use>             ::= "use" <module-ref> <import-pattern-tail>        ; (7.5)
<module-ref>      ::= <string> | "null" | <variable> | "(" <block-tokens> ")"   ; const-evaluated
; nu: parse_module.rs::parse_use; parse_signatures.rs::parse_import_pattern

<module>          ::= "module" <module-name> [ "{" <module-body> "}" ]
<module-name>     ::= <string>                 ; a literal only: "{...}" is a type error,
                                               ; "$x", "(..)" and $"..." are "not a string"
; with "--help" or "-h" anywhere before a "--" the statement is a help call, in which
; nu parses the name as a <string> and never the item in the body's place: `module x
; {ls} --help` and `module x --help {}#` parse, a third item is ExtraPositional
; nu: parse_module.rs::parse_module; parse_calls.rs::parse_internal_call

<module-body>     ::= { <module-item> }         ; lexed as it is, with no brace probe (5.13):
                                               ; `module m { |def x [] {} }` has a leading pipe
<module-item>     ::= <def> | <extern> | <export> | <attribute-block>
                    | <const> | <alias> | <use> | <module> | <export-env>
; anything else (`let`, `mut`, a call, `if`, a pipeline of > 1 command) is
; ExpectedKeyword "def, const, extern, alias, use, module, export or export-env"
; nu: parse_module.rs::parse_module_block

<export>          ::= "export" ( <def> | <extern> | <alias> | <use> | <module> | <const> )
; anything else after "export" (nothing, `let`, `mut`) is an error; `export def --env`
; works (the flags belong to the def)
; nu: parse_module.rs::parse_export_in_block, parse_module.rs::parse_export_in_module

<export-env>      ::= "export-env" <block> { <item> }
; only the first argument is looked at: `export-env {} extra`, `export-env {} {}` and
; a redirection on it parse in nu, the rest dropped. A closure (`{|x| }`) or a record
; body is refused.
; nu: parse_module.rs::parse_export_env

<hide>            ::= "hide" <string> [ <member> ]             ; module or single decl
; hide-env [--ignore-errors | -i] { <string> } is an ordinary command
<source>          ::= "source" ( <filepath> | "null" )
<source-env>      ::= "source-env" ( <string> | "null" )
<run>             ::= "run" ( <filepath> | "null" ) { <value> } [ "--full-reparse" | "-f" ]
<overlay-stmt>    ::= "overlay" "list"
                    | "overlay" "new" <string> [ "--reload" | "-r" ]
                    | "overlay" "use" ( <string> | "null" ) [ "as" <string> ]
                                      { "--prefix" | "-p" | "--reload" | "-r" }
                    | "overlay" "hide" [ <string> ] { "--keep-custom" | "-k"
                                      | ( "--keep-env" | "-e" ) <list> }
<plugin-use>      ::= "plugin" "use" <string> [ "--plugin-config" <filepath> ]
; source/source-env take exactly one positional, which must const-evaluate; `overlay`
; alone or `overlay foo` are errors; `as` without a name is KeywordMissingArgument;
; module, overlay and file names are const-evaluated and resolved against the file
; system or the module table.
; nu: parse_module.rs::parse_hide, parse_module.rs::parse_overlay_new,
;     parse_module.rs::parse_overlay_use, parse_module.rs::parse_overlay_hide;
;     parse_source.rs::parse_source, parse_source.rs::parse_run,
;     parse_source.rs::parse_plugin_use; nu-cmd-lang core_commands signatures
```

### 6.6 Control flow

```ebnf
<for>             ::= "for" <var-decl> "in" <value> <block>
; the iterable is ONE item of shape Any (a bare word is a string); the body is a
; Block (no params, no record); no flags (`--numbered` was removed); nothing may
; follow the block. The last item is reserved for the block before the keyword's
; argument is read, so `for x in []` and `for x --help in []` are
; KeywordMissingArgument("in") (a help flag forgives nothing there); the type of
; a `x:` variable is every item before "in" (7.4).
; nu: parse_def.rs::parse_for; nu-cmd-lang core_commands/for_.rs;
;     parse_calls.rs::calculate_end_span, parse_calls.rs::parse_multispan_value

<while>           ::= "while" <math-expression> <block>
<loop>            ::= "loop" <block>
; the condition is every item before the last; a record body is a type error
; unless nu types it any (5.13 <block>)
; nu: nu-cmd-lang core_commands/while_.rs, core_commands/loop_.rs (parsed by parse_call)

<if>              ::= "if" <math-expression> <block> [ "else" <else-body> ]
<else-body>       ::= <block> | <expression>         ; OneOf(Block, Expression)
; the condition is every item before the then-block, which is the item before
; "else" or the last item; "else" takes ALL remaining items as one expression, so
; `else if ...`, `else 5`, `else ls` and `else {|x| }` (Block fails, Expression gives
; a closure) parse; `else {} else {}` is an error; a record then-block is a type error
; nu: nu-cmd-lang core_commands/if_.rs; parse_calls.rs::parse_multispan_value (Keyword + OneOf)

<match>           ::= "match" <value> <match-block>
                    | "match" <value> ( <closure> | <record> | <variable> | "(" <block-tokens> ")" )
; The second form is what nu makes of a `{|x| ..}`, `{a: 1}`, `$x` or `(..)` where
; the arms should be: the MatchBlock shape lets the brace probe (5.13) and the "$"/"("
; dispatch through, the value type-checks as Any, and the match fails at run time.
; `match 1 [a]` and `match 1 foo` are errors.
<match-block>     ::= "{" { <arm-sep> } { <match-arm> { <arm-sep> } } "}"
<arm-sep>         ::= <whitespace> | "," | <eol> | "|" | <comment>
<match-arm>       ::= <pattern> { "|" <pattern> } [ "if" <math-expression> ] "=>" <arm-body>
<arm-body>        ::= <block> | <expression-of-one-item>     ; OneOf(Block, Expression)
; the guard is every token between "if" and the next "=>"; the body is exactly ONE
; item (`=> ls -l` makes `-l` the next arm's pattern); `{a: 1}` and `{|x| ..}` bodies
; parse (record / closure); a scrutinee bare word is a string. "|" between arms is
; whitespace to the lexer (so `1 => 2 | 3 => 4` fails only type checking).
; nu: parse_expressions.rs::parse_match_block_expression, parse_expressions.rs::parse_value;
;     nu-cmd-lang core_commands/match_.rs

<try>             ::= "try" <block> [ <handler> ] [ <handler> ]
<handler>         ::= ( "catch" | "finally" ) <closure-value>
<closure-value>   ::= <closure> | <variable> | "(" <block-tokens> ")"   ; must be a closure (type)
; both optional slots accept either keyword: `finally {} catch {}` and even
; `catch {} catch {}` parse; a third handler is ExtraPositional; a bare word or a
; record whose keys are all plain strings is an error (5.13 <block>)
; nu: nu-cmd-lang core_commands/try_.rs (OneOf(Keyword catch, Keyword finally) twice)

<return>          ::= "return" [ <value> ]
<break>           ::= "break"
<continue>        ::= "continue"
; `return` takes ONE item of shape Any (`return 1 2`, `return 1 + 1` are extra
; positionals; `return -1` is "doesn't have flag -1"); break/continue take nothing
; nu: nu-cmd-lang core_commands/return_.rs, core_commands/break_.rs,
;     core_commands/continue_.rs

<where>           ::= "where" <row-condition>                 ; (7.7)
; nu: parse_source.rs::parse_where, parse_source.rs::parse_where_expr

; collect [<closure>] [--keep-env] is an ordinary command; it is marked a keyword
; only for `$in` handling
; nu: nu-cmd-lang core_commands/collect.rs
```

### 6.7 Attributes

```ebnf
<attribute-block> ::= 1*<attribute> <annotated>
<attribute>       ::= "@" <attr-name> <args> ( <eol> | ";" )        ; lite rules in 2
<attr-name>       ::= <word> { " " <word> }      ; resolved as the command `attr <name>`
<annotated>       ::= <def> | <extern> | "export" ( <def> | <extern> )
; `@` alone or `@ name` gives an empty name (UnknownCommand); the name must resolve
; to a declaration `attr <name>`; the definition must be on the very next line;
; anything else (`export alias`, `export const`, `export module`, `export use`,
; `let`, a call) is AttributeRequiresDefinition
; nu: parse_calls.rs::parse_attribute; parse_def.rs::parse_attribute_block;
;     parse_module.rs::parse_export_in_block, parse_module.rs::parse_export_in_module
```

## 7. Signatures, types, patterns, import patterns, row conditions

### 7.1 Signatures

```ebnf
<params>          ::= { <param-separator> } { <param> { <param-separator> } }
; lexed with "\n\r" as whitespace, ":" "=" "," special and in_signature, comments kept
; nu: parse_signatures.rs::parse_signature_helper

<param-separator> ::= "," | <eol> | "|" | "||" | ";" | <redirection-op>
; every non-item token is ignored; "," twice, or "," followed by ":" / "=" / "(-x)",
; is "expected parameter or flag"
; nu: parse_signatures.rs::parse_signature_helper

<param>           ::= <param-head> [ ":" <param-type> ] [ "=" <default> ] { <description-comment> }
; mode machine Arg -> (":") Type -> AfterType -> ("=") DefaultValue -> Arg. ":" or "="
; as the LAST token is "expected type" / "expected default value", but a comment
; counts as a token, so `[x: # c\n]` is fine (no type) and `[x = # c\n y]` gives x the
; default `y`; a second "=" after a default is accepted (the second wins); a second
; ":" is an error; ":" or "=" with no parameter before them is silently skipped
; nu: parse_signatures.rs::parse_signature_helper

<param-head>      ::= <positional-param> | <optional-param> | <rest-param>
                    | <long-flag-param> | <short-flag-param> | <short-alias>
<positional-param> ::= <decl-name>
<optional-param>  ::= <decl-name> "?"                  ; checked before <rest-param>: "...x?" is invalid
<rest-param>      ::= "..." <decl-name>                 ; empty name: RestNeedsName; a second rest:
                                                        ; MultipleRestParams; a default: error
<long-flag-param> ::= "--" <flag-name> [ "(-" <short-char> ")" ]
; split on "("; the part after must be "-" <one char> ")"; the variable name is
; <flag-name> with "-" -> "_" and must be an identifier. "--" alone is NOT a long
; flag (it falls through to <short-flag-param> with the invalid name "-").
<short-flag-param> ::= "-" <short-char>                 ; the char must be an identifier byte
<short-alias>     ::= "(-" <short-char> ")"             ; a separate item after a flag: `--long (-l)`;
                                                        ; errors after ",", when the flag already has
                                                        ; a short form, or with no preceding flag
<flag-name>       ::= 1*( <identifier-byte> | "-" )
<decl-name>       ::= [ "$" ] 1*<identifier-byte>       ; digits, "?", ":", "@", "#" are allowed
; nu: parse_signatures.rs::parse_signature_helper; parse_helpers.rs::is_variable

<param-type>      ::= <shape> [ "@" <completer> ]
; split at the FIRST "@" in the token regardless of "<" ">" nesting (so
; `record<a@b: int>` is an unclosed "<"); an empty shape before "@" is UnknownType;
; "bool" on a flag is "Type annotations are not allowed for boolean switches"; a
; rest param's type becomes list<shape>
; nu: parse_signatures.rs::parse_signature_helper
<completer>       ::= <value>
; parsed with OneOf(list<string>, string) and const-evaluated: a string must name an
; existing command, a list is a static list; a "(..)" or "{..}" is an error
; nu: parse_shape_specs.rs::parse_completer
<default>         ::= <value>
; parsed with the DECLARED shape (`x: int = abc`, `x: int = "a"`, `x: bool = 1`,
; `x: list<int> = 1` are parse errors; `x: string = 1` is the string "1"), then
; const-evaluated; an untyped flag with a default takes the default's type
; nu: parse_signatures.rs::parse_signature_helper; parse_expressions.rs::parse_value
<description-comment> ::= <comment>
; attaches to the most recent parameter; several are joined with "\n"
; nu: parse_signatures.rs::parse_signature_helper
```

Checks applied while the parameters are collected, all in
`parse_signatures.rs::parse_signature_helper`: RequiredAfterOptional and
MultipleRestParams; the reserved names `in`, `nu`, `env` and `ans`
(`parse_signatures.rs::ensure_not_reserved_variable_name`, skipped for
`extern`); the boolean-switch type rule; and NonConstantDefaultValue, which
needs constant evaluation of the default.

### 7.2 Type annotations (shapes)

```ebnf
<shape>           ::= "any" | "binary" | "bool" | "cell-path" | "closure" | "datetime"
                    | "directory" | "duration" | "error" | "external_arg" | "float"
                    | "filesize" | "glob" | "int" | "nothing" | "number" | "path"
                    | "range" | "string"
                    | "list"   [ "<" <shape-list> ">" ]
                    | "record" [ "<" <named-shapes> ">" ]
                    | "table"  [ "<" <named-shapes> ">" ]
                    | "oneof"  [ "<" <shape-list> ">" ]
; "block" is the explicit error "Use 'closure' instead of 'block'"; every other word
; is UnknownType (with a hint when it contains "@"). Types: path/directory -> string,
; glob -> glob, datetime -> date, external_arg -> any.
; nu: parse_shape_specs.rs::parse_shape_name, parse_shape_specs.rs::parse_type

; The "<" ... ">" of a generic type is split at the FIRST "<"; the token must END with ">" (a ">" elsewhere is "Extra
; characters", none is Unclosed ">")
; nu: parse_shape_specs.rs::split_generic_params

<shape-list>      ::= { "," } [ <shape> { { "," } <shape> } { "," } ]
; "," tokens are skipped wherever they occur; list<> with more than one shape is
; "expected a single type parameter"; `list` / `list<>` is list<any>
; nu: parse_shape_specs.rs::parse_type_params

<named-shapes>    ::= { "," } { <named-shape> { "," } }
<named-shape>     ::= <field-name> [ ":" <shape> ]      ; no ":" means any
<field-name>      ::= <string>                          ; bare, quoted, numbers are fine
; "," tokens are skipped anywhere (",," and a leading "," are fine); ":" as the last
; token is "expected type after colon"
; nu: parse_shape_specs.rs::parse_named_type_params
```

### 7.3 Input/output types

```ebnf
<io-types>        ::= <io-pair> { "," <io-pair> }
                    | "[" { <io-pair> [ "," ] } "]"
<io-pair>         ::= <shape> "->" <shape>
; a leading "[" and a trailing "]" are stripped independently; the rest is lexed with
; "\n\r," as whitespace, comments skipped, in_signature; an empty list is fine;
; a missing "->" is Expected "arrow (->)"; a missing output shape is MissingType
; nu: parse_signatures.rs::parse_input_output_types
```

### 7.4 Variable declarations with an optional type

```ebnf
<var-decl>        ::= <decl-name>
                    | <decl-name> ":" <type-items>
<type-items>      ::= <shape>
; the ":" must be GLUED to the name: `let x : int` is ExtraTokens, and `x:int` (no
; space) is a variable called "x:int" since ":" is an identifier byte. A name with a
; space or quote is VariableNotValid; "x:" as the last item before "=" is MissingType.
; Every item between "x:" and "=" (for `let`/`mut`/`const`) or "in" (for `for`, where
; the positional ends at the keyword) is concatenated and re-lexed as one
; signature-mode span (so `record<a: int, b: string>` with spaces works); only the
; first token is the type, so `for x: int --help in [] {}` and `let x: int --help =
; 1` are UnknownType("int --help"). Reserved names are NameIsBuiltinVar.
; nu: parse_signatures.rs::parse_var_with_opt_type, parse_signatures.rs::ensure_not_reserved_variable_name; parse_bindings.rs::parse_let;
;     parse_calls.rs::calculate_end_span
```

### 7.5 Import patterns

```ebnf
<import-pattern-tail> ::= { <member-name> } [ <glob> | <member-list> ]
<member-name>     ::= <string>                ; several names form a path through submodules
<glob>            ::= "*"                     ; only last
<member-list>     ::= "[" { <string> } "]"    ; only last; non-string items are silently
                                              ; dropped; a spread is WrongImportPattern
; a member that is not a string, "*" or a list (`null`, `5`, `{..}`) is
; WrongImportPattern; a `$var` member is SILENTLY IGNORED; anything after "*" or
; "[..]" is "... member can be only at the end of an import pattern"; a cell path
; on a list member (`[math].x`) is ignored
; After `use null` (a no-op) the members are parsed and then never looked at.
; nu: parse_signatures.rs::parse_import_pattern; parse_module.rs::parse_use
```

### 7.6 Match patterns

```ebnf
<pattern>         ::= "_"
                    | <variable-pattern>
                    | <record-pattern>
                    | <list-pattern>
                    | <value-pattern>
; dispatch on the first byte ("$", "{", "[") or the exact word "_"
; nu: parse_patterns.rs::parse_pattern

<variable-pattern> ::= "$" 1*<identifier-byte>       ; reserved names are NameIsBuiltinVar
; nu: parse_patterns.rs::parse_variable_pattern

<value-pattern>   ::= <value>
; parse_value(Any) then eval_constant: literals, ranges, lists and records of
; constants, "(const subexpression)"; a non-constant is a PARSE error. "..foo" is a
; plain string value pattern.
; nu: parse_patterns.rs::parse_value_pattern

<list-pattern>    ::= "[" { <list-pattern-item> } "]"
; lexed with "\n\r," as whitespace, then LITE-PARSED: ";" is an explicit error, "|"
; splits the items into commands and is dropped silently
<list-pattern-item> ::= ".."                  ; IgnoreRest; parsing STOPS here, later items are dropped
                    | "..$" 1*<identifier-byte>   ; Rest(var); parsing stops here too
                    | <pattern>
; a redirection token is dropped as in a list (5.14); an assignment-op item is a
; string pattern and, as in every lite parse, so is everything after it
; nu: parse_patterns.rs::parse_list_pattern

<record-pattern>  ::= "{" { <record-pattern-item> } "}"
<record-pattern-item> ::= "$" 1*<identifier-byte>          ; shorthand: field bound to $name
                    | <field-token> ":" <pattern>
; lexed with "\n\r," as whitespace and ":" special; EVERY token is a field token,
; and the one after it must be ":" (so `{a: 1; b: 2}` is "expected record" because
; the field ";" is followed by `b`, while `{; : 1}` parses); the field text is taken
; VERBATIM (quotes are not stripped: `{"a": $x}` never matches a field `a`)
; nu: parse_patterns.rs::parse_record_pattern
```

### 7.7 Row conditions

```ebnf
<row-condition>   ::= <closure>
                    | <math-expression>
; the whole span is first tried as a closure (`{|x| ..}`, `{ .. }`, `{}`); otherwise,
; as for a brace item with a tail (`{}.a`, `{a: 1}.a`, 5.13), it is a math expression with a fresh `$it` in scope in which every bare-string
; operand, also under `not`, becomes a cell path on `$it` (`size` -> `$it.size`).
; A plain `$var` or a cell path on another variable is returned as-is (a closure at
; run time); anything else must be bool-typed (TypeMismatch otherwise). `where` is
; parsed through its command signature, so `where` alone is MissingPositional and
; flags are flags.
; nu: parse_signatures.rs::parse_row_condition, parse_signatures.rs::expand_to_cell_path;
;     nu-command filters/where_.rs
```

## 8. Keeping this file true

The ```` ```ebnf ```` fences are the grammar; the prose around them is
commentary. `grammar.bnf` and `grammar.ebnf` in this directory are generated
from the fences by `gen-grammar.nu` and are never edited by hand:

```nushell
nu devdocs/grammar/gen-grammar.nu           # rewrite grammar.bnf and grammar.ebnf
nu devdocs/grammar/gen-grammar.nu --check   # exit 1 when either file is stale
```

`grammar.bnf` is the fences verbatim, in order, with the section headings as
comments. `grammar.ebnf` is the same grammar in ISO 14977 notation, which
[ebnf2railroad](https://www.npmjs.com/package/ebnf2railroad) and the VS Code
EBNF extension understand. The lint reports undefined or unused rules; the
render writes a page of railroad diagrams, which is not committed:

```nushell
ebnf2railroad --lint devdocs/grammar/grammar.ebnf --no-target
ebnf2railroad --title "Nushell Grammar" devdocs/grammar/grammar.ebnf -o grammar.html
```

When nu-parser changes: update the affected rule and its `; nu:` line, run
`gen-grammar.nu`, and move the commit and date in the first paragraph forward.
`gen-grammar.nu` reads them from that paragraph (the text ``commit `hash`,
DATE``) and writes them into the header of both generated files. A new
`SyntaxShape`, `TokenContents` or `ParseError` variant, a new keyword command
in nu-cmd-lang or a new `parse_*` function in nu-parser usually means a rule is
missing here or has changed.

## Appendix A. Character classes and shared vocabulary

Names used by several sections, defined once here (after the grammar so that
`<source-file>` is the first rule, the start symbol, in the derived files). Nushell source is bytes;
"byte" means one byte of UTF-8.

```ebnf
<digit>           ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
<ascii-digit>     ::= <digit>
<ascii-letter>    ::= <a byte in "A".."Z" or "a".."z">
<digit-2>         ::= "0" | "1"
<digit-8>         ::= <digit-2> | "2" | "3" | "4" | "5" | "6" | "7"
<digit-10>        ::= <digit>
<digit-16>        ::= <digit> | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F"
<hex>             ::= <digit-16>
<radix-digit>     ::= <digit-2> | <digit-8> | <digit-16>      ; the digits of the literal's own radix
<char>            ::= <one UTF-8 encoded character>
<short-char>      ::= <char>                 ; for <short-flag-param> it must also be an <identifier-byte>
<any-chars>       ::= { <any byte> }
<additional-whitespace> ::= <the bytes the construct being lexed treats as whitespace (1.4)>
<bare-char>       ::= <any byte the lexer lets into an item in the current context (1.3)>
<dq-char>         ::= <any byte except "\">
<dq-text>         ::= { <dq-char> | <escape> }                ; text between the "(...)" parts of $"..."
<sq-text>         ::= { <any byte except "'"> }
<bare-text>       ::= { <bare-char> }                         ; text between the "(...)" parts of a bare word
<bare-glob>       ::= 1*<bare-char>                           ; an external word with no quote, paren or backtick
<shebang>         ::= "#!" { <any byte except "\n"> }
<word>            ::= <bare-word>                             ; one unquoted item (5.6)

; Kinds of item, named by their first byte (each is one <item>, 1.3).
<var-item>        ::= <dollar-expr>                           ; starts with "$" (5.9)
<paren-item>      ::= <paren-expr>                            ; starts with "(" (5.12)
<bracket-item>    ::= <bracket-expr>                          ; starts with "[" (5.1)
<brace-item>      ::= <brace-expr>                            ; starts with "{" (5.13)
<item-rest>       ::= <the text of an item after its first byte>

; Bodies of bracketed items (the text between the delimiters).
<list-body>       ::= { <list-sep> } { <list-item> { <list-sep> } }
<list-or-table-body> ::= <the interior of a list-or-table item (5.14)>
<record-body>     ::= { <record-sep> } { <record-entry> { <record-sep> } }

; Names used across sections.
<expression>      ::= <pipeline-element>                      ; parse_expression (3.1)
<expression-of-one-item> ::= <expression>                     ; restricted to exactly one item
<value-of-shape>  ::= <value>                                 ; parsed with the SyntaxShape of its argument position
<external-arg>    ::= <ext-arg>                               ; (4)
<keyword-word>    ::= <the keyword a SyntaxShape::Keyword names: "in", "else", "catch", "finally", "as">
<builtin-head>    ::= <call-head>                             ; must name a built-in command
<filepath>        ::= <string>                                ; a file path (semantic)
<field-token>     ::= <bare-word> | <double-quoted> | <single-quoted> | <backtick-quoted>   ; text taken verbatim
<where-expr>      ::= <where>                                 ; (6.6, 7.7)
<run-expr>        ::= <call>                                  ; "run" parsed as an ordinary call
<statement-in-pipeline-error> ::= <pipeline-forbidden-head>        ; (6.1)
```
