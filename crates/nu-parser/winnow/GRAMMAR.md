# The winnow parser and the Nushell grammar

The Nushell grammar lives in [nushell/grammar](https://github.com/nushell/grammar). Its
`grammar.md` describes the language as nu-parser accepts it, rule by rule, and its corpus and
harness check the grammar against nu-parser. That repository says nothing about this parser.

This file is this parser's side of the grammar:

* where each rule is implemented here,
* what a consumer adds for the rules that need more than the text, and
* the verified differences between this parser and nu-parser.

Section numbers and rule names are the grammar's. Sections 1 to 7 of the grammar hold the
rules; its section 9 lists the inputs the text alone does not decide, which this parser leaves
to its consumer. The tables below were split out of the grammar when it moved to its own
repository; the rules then cited nushell `d7cb4e589`.

## Reading the annotations

| annotation | meaning |
| --- | --- |
| `path::fn` | where this parser implements the rule (`src/winnow/...`, relative to `crates/nu-parser`) |
| `consumer` | needs signatures, declarations, const-eval or files: left to the consumer by design |
| `none` | not implemented |
| `DIFF n` | a verified disagreement with nu-parser, see section 9 below |

## Where each rule is implemented

A row names the grammar rules that share one implementation. A rule stated in prose instead of
notation is named by the first words of its comment.

### 1.1 Layout

| rules | winnow |
| --- | --- |
| `<source-file>` | src/winnow/syntax/parser/mod.rs::parse (records `shebang`) |
| `<whitespace>` | src/winnow/syntax/lex.rs::skip_whitespace, item_length (is_terminator) |
| `<carriage-return>` | src/winnow/syntax/lex.rs::skip_whitespace, item_length |
| `<eol>` | src/winnow/syntax/lex.rs::lex_token, LexOptions::additional_whitespace |
| `<comment>` | src/winnow/syntax/lex.rs::lex_comment, item_length (`in_comment`, `previous`) |

### 1.2 Tokens

| rules | winnow |
| --- | --- |
| `<token-stream>`, `<token>` | src/winnow/syntax/lex.rs::TokenContents, lex_token |
| `<pipe>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock (the same rule, applied in the lite parse: exactly `Eol (Comment Eol)* Pipe` continues the line) |
| `<pipe-pipe>` | src/winnow/syntax/lex.rs::lex_token, src/winnow/syntax/parser/lite_parser.rs::LiteBlock |
| `<semicolon>` | src/winnow/syntax/lex.rs::lex_token, src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline |
| `<assignment-op>` | src/winnow/syntax/lex.rs::item_contents, AssignmentOperator |
| `<redirection-op>`, `<out-redirect>`, `<err-redirect>`, `<out-err-redirect>`, `<err-pipe>`, `<out-err-pipe>` | src/winnow/syntax/lex.rs::item_contents, is_redirection, RedirectionOperator |
| `<bashism>` | src/winnow/syntax/lex.rs::item_contents (ErrorKind::ShellSyntax) |

### 1.3 Items

| rules | winnow |
| --- | --- |
| `<item>`, `<item-part>` | src/winnow/syntax/lex.rs::item_length, lex_item |
| `<plain-byte>` | src/winnow/syntax/lex.rs::item_length |
| `<quoted-run>`, `<dq-byte>` | src/winnow/syntax/lex.rs::item_length (`quote` state) |
| `<interp-quoted-run>` | src/winnow/syntax/lex.rs::item_length (`quote_is_interp`, `interp_level`), interp_subexpr_step |
| `<raw-string>`, `<hashes>` | src/winnow/syntax/lex.rs::lex_raw_string |
| `<bracketed-run>`, `<nested>` | src/winnow/syntax/lex.rs::item_length, close_bracket, group_end |


### 1.4 Re-lexing options per construct

nu-parser re-lexes the interior of a bracketed item with options that depend on the construct;
this parser's `LexOptions` constants carry the same sets.

| construct | nu | here |
| --- | --- | --- |
| block, file | parse_block | `BLOCK` |
| subexpression `( )`, `(...)` cell-path head | parse_paren_expr, parse_full_cell_path | `SUBEXPRESSION` |
| list `[ ]` | parse_list_expression | `LIST` |
| record `{ }` key | parse_record | `RECORD_KEY` |
| record `{ }` value | parse_record | `RECORD_VALUE` |
| brace probe (first two tokens of `{ }`) | parse_brace_expr | `BRACE_PROBE` |
| signature `[ ]` `( )` `\| \|` | parse_signature_helper | `SIGNATURE` |
| input/output types | parse_input_output_types | `IO_TYPES` |
| generic type params `< >` | parse_type_params | `IO_TYPES` / `SIGNATURE` |
| cell path | parse_cell_path | `CELL_PATH` |
| match block | parse_match_block_expression | `MATCH` (`\|` is a token here, whitespace in nu: the same arms) |
| list pattern | parse_list_pattern (then lite-parsed) | `PATTERN_LIST` |
| record pattern | parse_record_pattern | `PATTERN_RECORD` |
| binary literal | parse_binary_with_base | `BINARY` |
| assignment rhs | parse_assignment_expression | block parse of the token slice |
| `let x: <type>` items | parse_var_with_opt_type | `parse_type` on the merged span |

### 2. Blocks, pipelines and commands (the lite parse)

| rules | winnow |
| --- | --- |
| `<block-tokens>`, `<separator>`, `<comment-line>` | src/winnow/syntax/parser/parse_pipelines.rs::parse_block |
| `<pipeline>`, `<lite-pipeline>` | src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline (a pipeline of only pipes yields nothing) |
| `<pipe-continuation>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock, last_non_comment_token |
| the rule "The trailing-pipe rule: a BLOCK whose last non-comment ..." | src/winnow/syntax/parser/parse_pipelines.rs::parse_block, src/winnow/syntax/parser/lite_parser.rs::last_non_comment_token (nu's, ported) |
| `<command>`, `<command-body>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock |
| `<redirection>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock, src/winnow/syntax/parser/parse_pipelines.rs::parse_redirection |
| `<pipe-redirection>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock, src/winnow/syntax/parser/parse_pipelines.rs::parse_redirection |
| `<assignment-tail>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock (the same continuation rule) |
| `<attribute-lines>`, `<attribute-line>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock (every token of the line becomes a word), src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline_element |

### 2.1 Which parser sees a command

| rules | winnow |
| --- | --- |
| `<parsed-pipeline>`, `<single-command-pipeline>` | src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline_element, src/winnow/syntax/parser/parse_expressions.rs::parse_builtin_commands (one match for all keywords including if/match/...; source, hide, run, overlay, plugin use are plain Calls, their arguments the consumer's but for hide's import pattern and overlay use's `as`: parse_calls.rs::check_call) |
| `<multi-command-pipeline>` | src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline (the commands are collected first, then parsed in Position::Element), src/winnow/syntax/parser/parse_expressions.rs::check_builtin_command_in_pipeline |

### 3.1 Expression dispatch (`parse_expression`)

| rules | winnow |
| --- | --- |
| `<pipeline-element>`, `<env-shorthand>`, `<env-name>`, `<env-value>` | src/winnow/syntax/parser/parse_expressions.rs::parse_env_shorthand_prefix, is_env_variable_name |
| `<element-body>` | src/winnow/syntax/parser/parse_expressions.rs::parse_expression (Position::Element after a "\|" and after shorthand), check_builtin_command_in_pipeline (the same head table) |
| the rule "An item is math-expression-like when it is one ..." | src/winnow/syntax/parser/parse_expressions.rs::is_math_expression_like, src/winnow/syntax/parser/parse_literals.rs::is_range_head |

### 3.2 Math expressions and operators

| rules | winnow |
| --- | --- |
| `<math-expression>`, `<keyword-expression>`, `<operand>`, `<negated>`, `<tight-operator>`, `<operand-or-keyword>` | src/winnow/syntax/parser/parse_expressions.rs::parse_math_expression, parse_math_operand, negated_operand_end, fold_binary_op |
| `<operator>` | src/winnow/syntax/ast/mod.rs::Operator::from_spelling, src/winnow/syntax/parser/parse_expressions.rs::parse_operator |

### 3.3 Assignment

| rules | winnow |
| --- | --- |
| `<assignment>`, `<assign-lhs>`, `<assign-rhs>` | src/winnow/syntax/parser/parse_expressions.rs::parse_assignment_expression (lhs: Var, Subexpression, FullCellPath, List, Table or a "key: value" Record, which are what parse_full_cell_path yields; the "^" rule is applied when a command table is configured) |

### 4. Calls

| rules | winnow |
| --- | --- |
| `<call>` | src/winnow/syntax/parser/parse_calls.rs::parse_call |
| `<keyword-command>` | src/winnow/syntax/parser/parse_expressions.rs::parse_builtin_commands (the longer name through parse_calls.rs::find_longest_decl, then parse_call) |
| `<external-call>`, `<unknown-head>` | src/winnow/syntax/parser/parse_calls.rs::parse_call_lenient, parse_external_arguments; working_set.rs::find_decl (DeclKind::ExternalAlias) |
| `<ext-head>`, `<ext-arg>` | src/winnow/syntax/parser/parse_calls.rs::parse_external_call, parse_external_arg, check_external_list_argument |
| `<ext-string>`, `<segment>` | src/winnow/syntax/parser/parse_calls.rs::parse_external_string (glob-vs-string is not represented: both are String/StringInterpolation; consumer) |
| `<sigil-call>` | src/winnow/syntax/parser/parse_calls.rs::parse_percent_call (find_longest_name asking find_builtin_decl: the built-in table from ParseConfig, or the engine's CommandLookup::find_builtin_decl) |
| `<internal-call>`, `<call-head>` | src/winnow/syntax/parser/parse_calls.rs::find_longest_decl (longest known name, at most 5 words with a ParseConfig table, any number with an engine's CommandLookup, from the table plus def/extern/alias names declared in scope; alias subcommand expansion: consumer) |
| `<args>`, `<arg>` | src/winnow/syntax/parser/parse_calls.rs::parse_call_arguments |
| `<long-flag>` | src/winnow/syntax/parser/parse_calls.rs::parse_call_arguments (`--x`, `--x=v`; whether `--x v` binds v: consumer) |
| `<short-flags>` | src/winnow/syntax/parser/parse_calls.rs::parse_call_arguments (`-<digit>`, `-.<digit>` always a positional number, `-..` a range; consumer) |
| `<spread-arg>` | src/winnow/syntax/parser/parse_calls.rs::parse_call_arguments, src/winnow/syntax/parser/parse_helpers.rs::is_spread; consumer |
| `<positional>` | src/winnow/syntax/parser/parse_expressions.rs::parse_value with ExpectedShape::Any for every positional; the multi-span shapes only occur in keyword statements, where each statement parser handles them (6); consumer otherwise (nu-parser's front end: src/winnow/lower/call.rs::multispan_value, which has a row condition or a math expression of several items parsed again with parser/mod.rs::parse_items) |
| `<keyword-arg>` | inside each statement parser (`in`, `else`, `catch`, `finally`, `as`, `=`) |

### 5.1 Dispatch

| rules | winnow |
| --- | --- |
| `<value>` | src/winnow/syntax/parser/parse_expressions.rs::parse_value |
| `<bracket-expr>` | src/winnow/syntax/parser/parse_expressions.rs::parse_value, parse_value_for_shape ("[" checked against the shape for the typed defaults of 7.1, and for the arguments of calls whose signature the parser knows: no lookup, a built-in or a `def` of the text) |
| `<negation>` | src/winnow/syntax/parser/parse_expressions.rs::parse_negation, is_negation |
| `<shaped-value>`, `<negation>` | src/winnow/syntax/parser/parse_expressions.rs::parse_value with ExpectedShape::{Any,Number,String,Closure,MatchArmBody,Declared}: Declared is the declared shape of a parameter default (7.1), or of a call's parameter when the parser knows the signature (no lookup: a built-in or a `def` of the text, parse_calls::parse_call_arguments_by_shape); otherwise the consumer's |
| `<any-value>` | src/winnow/syntax/parser/parse_expressions.rs::parse_any_value (same order) |

### 5.2 Booleans, nothing, numbers

| rules | winnow |
| --- | --- |
| `<nothing>` | src/winnow/syntax/parser/parse_expressions.rs::parse_any_value |
| `<bool>`, `<nothing>`, `<int>`, `<int-text>` | src/winnow/syntax/parser/parse_literals.rs::read_int (IntLiteral), parse_int; src/winnow/syntax/parser/parse_expressions.rs::parse_any_value |
| `<float>`, `<float-text>` | src/winnow/syntax/parser/parse_literals.rs::parse_float, SPECIAL_FLOAT_NAMES, parse_number |

### 5.3 Filesize and duration

| rules | winnow |
| --- | --- |
| `<unit-number>`, `<filesize>`, `<filesize-unit>`, `<duration>`, `<duration-unit>` | src/winnow/syntax/parser/parse_literals.rs::parse_unit_value, parse_filesize, parse_duration (same tables, same order, UnitError) |

### 5.4 Datetime

| rules | winnow |
| --- | --- |
| `<datetime>`, `<date>`, `<time-sep>`, `<time>`, `<offset>` | src/winnow/syntax/parser/parse_literals.rs::is_datetime, days_in_month, is_invalid_date_literal |

### 5.5 Binary

| rules | winnow |
| --- | --- |
| `<binary>`, `<binary-prefix>`, `<binary-sep>` | src/winnow/syntax/parser/parse_literals.rs::parse_binary, parse_binary_with_base |

### 5.6 Strings

| rules | winnow |
| --- | --- |
| `<string>` | src/winnow/syntax/parser/parse_literals.rs::parse_string, parse_string_literal |
| `<bare-word>` | src/winnow/syntax/parser/parse_literals.rs::parse_string_literal (Quote::Bare) |
| `<double-quoted>`, `<single-quoted>`, `<backtick-quoted>` | src/winnow/syntax/parser/parse_literals.rs::quoted_string_body, parse_string_literal |
| `<escape>` | src/winnow/syntax/parser/parse_literals.rs::unescape_string (bytes, then one UTF-8 check) |
| the rule "\<raw-string> and \<hashes> are defined with the lexer ..." | src/winnow/syntax/parser/parse_literals.rs::parse_raw_string |

### 5.7 Interpolation

| rules | winnow |
| --- | --- |
| `<interpolation>`, `<bare-interpolation>`, `<interp-expr>` | src/winnow/syntax/parser/parse_literals.rs::parse_string_interpolation, parse_interpolation_parts; src/winnow/syntax/parser/parse_calls.rs::parse_external_string (bare words of external calls) |

### 5.8 Variables

| rules | winnow |
| --- | --- |
| `<variable>`, `<var-name>`, `<identifier-byte>` | src/winnow/syntax/parser/parse_helpers.rs::is_identifier, src/winnow/syntax/parser/parse_literals.rs::parse_variable_expr |

### 5.9 `$` items

| rules | winnow |
| --- | --- |
| `<dollar-expr>` | src/winnow/syntax/parser/parse_literals.rs::parse_dollar_expr |

### 5.10 Ranges

| rules | winnow |
| --- | --- |
| `<range>`, `<range-op>`, `<bound>` | src/winnow/syntax/parser/parse_literals.rs::is_range_syntax, find_range_operators, range_bounds, is_range_bound (a "{" bound through src/winnow/syntax/parser/parse_expressions.rs::BraceShape::of_probe), parse_range (it refuses a raw string, record or closure bound and a bare interpolation: a "(" bound is a closed group with any cell path after it, and a "(" group that does not close, `(1)abc`, is the string the ".." refuses; the type of any other bound, such as "$.a", is the consumer's), is_range_head (a command head, 3.1) |

### 5.11 Cell paths

| rules | winnow |
| --- | --- |
| `<cell-path-literal-body>`, `<full-cell-path>`, `<cell-path-head>`, `<cell-path-tail>`, `<member-sep>`, `<member>` | src/winnow/syntax/parser/parse_literals.rs::parse_cell_path, parse_dollar_expr, parse_full_cell_path, parse_simple_cell_path (a bare member with "(" in it is refused) |

### 5.12 Parenthesised items

| rules | winnow |
| --- | --- |
| `<paren-expr>` | src/winnow/syntax/parser/parse_literals.rs::parse_paren_expr, src/winnow/syntax/parser/parse_expressions.rs::parse_subexpression |

### 5.13 Brace items: record, closure, block

| rules | winnow |
| --- | --- |
| `<brace-expr>` | src/winnow/syntax/parser/parse_expressions.rs::parse_brace_expr, probe_brace_shape, BraceShape::of_probe (the classification, shared with range bounds, 5.10), parse_block_body (a "key:" body in block position is "expected block, found a record") |
| `<closure>`, `<closure-params>` | src/winnow/syntax/parser/parse_expressions.rs::parse_closure_expression |
| `<block>` | src/winnow/syntax/parser/parse_expressions.rs::parse_block_body, parse_block_expression, parse_brace_record, is_typed_record; src/winnow/syntax/parser/parse_keywords.rs::parse_block_or_value_argument, parse_brace_value |

### 5.14 Lists and tables

| rules | winnow |
| --- | --- |
| `<list-or-table>`, `<list>`, `<list-sep>`, `<list-item>`, `<spread>` | src/winnow/syntax/lex.rs::LexOptions::LIST (LexMode::ListItems), src/winnow/syntax/parser/lite_parser.rs::lite_parse_parts (the lite parse of a bracket's tokens; dropped redirections are recorded as ignored text), src/winnow/syntax/parser/parse_expressions.rs::parse_list_expression, parse_list_item; value.rs::is_spread |
| `<table>` | src/winnow/syntax/parser/parse_expressions.rs::parse_table_expression, parse_table_row, parse_field_name, check_duplicate_name (all the checks; a column name is a string when it is a string, an interpolation, a variable, a cell path or a subexpression: the type of the last three is the consumer's) |

### 5.15 Records

| rules | winnow |
| --- | --- |
| `<record>`, `<record-sep>`, `<record-entry>`, `<record-key>`, `<record-value>`, `<record-spread>` | src/winnow/syntax/parser/parse_expressions.rs::parse_record, parse_field_name, check_duplicate_name, check_record_key_or_value |

### 6.1 Keyword table and pipeline restrictions

| rules | winnow |
| --- | --- |
| `<keyword-statement>` | src/winnow/syntax/parser/parse_expressions.rs::parse_builtin_commands |
| the rule "Every keyword is a command with a fixed ..." | src/winnow/syntax/parser/parse_keywords.rs::keyword_boundary, KeywordCall::flags, KeywordCall::help_flag, seen_end_of_options, KeywordCall::positional, parse_help_call; src/winnow/syntax/parser/parse_helpers.rs::is_help_text |
| `<pipeline-forbidden-head>` | src/winnow/syntax/parser/parse_expressions.rs::check_builtin_command_in_pipeline (every element of a multi-command pipeline and the command after shorthand are parsed in Position::Element), src/winnow/syntax/parser/parse_pipelines.rs::check_loop_in_pipeline |
| the rule "Redirections are refused on: def, extern, alias, type, ..." | src/winnow/syntax/parser/parse_pipelines.rs::rejects_redirection (the statements, the keywords' calls, a lone let or mut, and the calls whose fixed signature is not redirectable); drops_redirection (export-env's and a const's redirection is ignored text) |
| `<parser-keyword>` | src/winnow/syntax/parser/parse_keywords.rs::is_parser_keyword (both tables, multi-word entries included) |
| `<bad-definition-name>` | src/winnow/syntax/parser/parse_def.rs::check_definition_name |
| the rule "a def/extern NAME refused either way (or with ..." | parse_def.rs::recover_name_error |
| the rule "Predeclaration: before a block is parsed, every single-command ..." | src/winnow/syntax/parser/parse_def.rs::parse_def_predecl, scan_predecls (TypeAliasPredecl, for the engine's BlockSink::predecl_type_alias; whether a pipe joins the next command: src/winnow/syntax/parser/lite_parser.rs::LiteBlock; the name: src/winnow/syntax/parser/parse_signatures.rs::parse_definition_name) |

### 6.2 let, mut, const

| rules | winnow |
| --- | --- |
| `<let>`, `<mut>`, `<const>`, `<rhs-pipeline>` | src/winnow/syntax/parser/parse_bindings.rs::parse_let |

### 6.3 def, extern

| rules | winnow |
| --- | --- |
| `<def>`, `<def-flag>`, `<definition-call>` | src/winnow/syntax/parser/parse_def.rs::parse_def, parse_definition_flags (before or after the name), definition_flag; ast::DefinitionFlag::is_set |
| `<def-name>`, `<reserved-definition-name>` | src/winnow/syntax/parser/parse_def.rs::parse_def_name, src/winnow/syntax/parser/parse_signatures.rs::parse_definition_name |
| `<full-signature>`, `<signature-item>`, `<signature-item-colon>` | src/winnow/syntax/parser/parse_signatures.rs::parse_full_signature (the dropped item is ignored text) |
| the rule "A signature error is nu's call argument error: ..." | parse_def.rs::signature_error_call, parse_calls.rs::parse_definition_call |
| `<def-body>` | src/winnow/syntax/parser/parse_def.rs::parse_def, parse_def_body (the `\|x\|` parameters are kept as Def::body_params), reads_as_call, parse_def_predecl and has_untyped_rest (DeclKind::Wrapped: the rest param from the lexed signature, its type from the text); src/winnow/syntax/parser/parse_calls.rs::parse_call_lenient (which records it as Call::wrapped) |
| `<extern>`, `<extern-flag>` | src/winnow/syntax/parser/parse_def.rs::parse_extern, parse_definition_flags (before or after the name), src/winnow/syntax/parser/parse_signatures.rs::parse_full_signature |

### 6.4 alias, type

| rules | winnow |
| --- | --- |
| `<alias>`, `<alias-target>` | src/winnow/syntax/parser/parse_alias.rs::parse_alias (the same three checks; missing positionals of a keyword target are forgiven as in nu, `alias x = overlay new`) |
| `<alias-target>` | DIFF 1: nu reads an `if`, `match` or `try` target through the keyword's signature (parse_call), this parser with the statement parser or as a call with flags: a missing positional nested in the target is not forgiven here (`alias i = if if x y`, 9.2) and some signature errors are missed (`alias i = try {} -- catch`, 9.1); a `%(...)` target is CantAliasKeyword in nu and accepted here (9.1) |
| `<type-alias>`, `<type-alias-name>`, `<type-alias-type>` | src/winnow/syntax/parser/parse_type_alias.rs::parse_type_alias, src/winnow/syntax/parser/parse_shape_specs.rs::{BUILTIN_TYPE_NAMES, is_valid_type_alias_name} |

### 6.5 Modules

| rules | winnow |
| --- | --- |
| `<use>`, `<module-ref>` | src/winnow/syntax/parser/parse_module.rs::parse_use, parse_import_pattern_member, parse_import_pattern_list |
| `<module>`, `<module-name>`, `<module-call>` | src/winnow/syntax/parser/parse_module.rs::parse_module, parse_module_name, parse_module_call (Argument::Unread for a garbage positional) |
| `<module-body>`, `<module-item>` | src/winnow/syntax/parser/parse_module.rs::is_module_item |
| `<export>` | src/winnow/syntax/parser/parse_module.rs::parse_export_in_block |
| `<export-env>` | src/winnow/syntax/parser/parse_module.rs::parse_export_env (the extra items and the redirection are ignored text) |
| `<hide>`, `<source>`, `<source-env>`, `<run>`, `<overlay-stmt>`, `<plugin-use>` | ordinary Call; the members of `hide` (an import pattern, unless the call shows help) and the keyword argument `as` (the positional after the module, followed by any item: `as --prefix` names the overlay; a flag, "-1" and a spread are no positionals): src/winnow/syntax/parser/parse_calls.rs::check_call; flags and their values, short-flag batches, positional counts, the types of the values, the const-evaluation and resolution of the names: consumer |

### 6.6 Control flow

| rules | winnow |
| --- | --- |
| `<for>` | src/winnow/syntax/parser/parse_def.rs::parse_for |
| `<while>`, `<loop>` | src/winnow/syntax/parser/parse_control_flow.rs::parse_while, parse_loop, src/winnow/syntax/parser/parse_keywords.rs::parse_block_argument |
| `<if>`, `<else-body>` | src/winnow/syntax/parser/parse_control_flow.rs::parse_if, parse_block_or_value |
| `<match>`, `<match-block>`, `<arm-sep>`, `<match-arm>`, `<arm-body>` | src/winnow/syntax/parser/parse_control_flow.rs::parse_match (the second form is kept as Match::value_block), src/winnow/syntax/parser/parse_expressions.rs::parse_match_block_expression, parse_match_arm |
| `<try>`, `<handler>`, `<closure-value>` | src/winnow/syntax/parser/parse_control_flow.rs::parse_try, parse_try_handler (the closure type of a variable or subexpression: consumer) |
| `<return>`, `<break>`, `<continue>` | src/winnow/syntax/parser/parse_control_flow.rs::parse_return, parse_break_or_continue, src/winnow/syntax/parser/parse_keywords.rs::KeywordCall (call_flags, end) |
| `<where>` | src/winnow/syntax/parser/parse_source.rs::parse_where |
| the rule "collect [\<closure>] [--keep-env] is an ordinary command; nu ..." | ordinary Call |

### 6.7 Attributes

| rules | winnow |
| --- | --- |
| `<attribute-block>`, `<attribute>`, `<attr-name>`, `<annotated>` | src/winnow/syntax/parser/lite_parser.rs::LiteBlock, src/winnow/syntax/parser/parse_pipelines.rs::parse_pipeline_element, src/winnow/syntax/parser/parse_calls.rs::parse_attribute (the empty name is the consumer's; whether `attr <name>` exists is the consumer's, it may come from a `use`d module; the arguments of the built-in attributes, as any call's, are the consumer's too) |

### 7.1 Signatures

| rules | winnow |
| --- | --- |
| `<params>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameters (LexOptions::SIGNATURE) |
| `<param-separator>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameters (every non-item token skipped) |
| `<param>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameters (same modes and skips; a later default without a declared type is parsed as any value; every default is kept, the earlier ones in Parameter::earlier_defaults: their types are the consumer's) |
| `<param-head>`, `<positional-param>`, `<optional-param>`, `<rest-param>`, `<long-flag-param>`, `<short-flag-param>`, `<short-alias>`, `<flag-name>`, `<decl-name>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameter, parse_parameters, check_parameters, check_unique_name |
| `<param-type>` | src/winnow/syntax/parser/parse_shape_specs.rs::parse_shape_name |
| `<completer>` | src/winnow/syntax/parser/parse_shape_specs.rs::parse_completer (a string or a list); whether the command exists (it may come from a `use`d module) and the const-evaluation: consumer |
| `<default>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameters via parse_value(ExpectedShape::Declared(shape)), src/winnow/syntax/parser/parse_expressions.rs::parse_value_for_shape; the const-evaluation: consumer |
| `<description-comment>` | src/winnow/syntax/parser/parse_signatures.rs::parse_parameters (Parameter::description keeps them all) |

### 7.2 Type annotations (shapes)

| rules | winnow |
| --- | --- |
| `<shape>` | src/winnow/syntax/parser/parse_shape_specs.rs::parse_type, parse_generic_shape (same list, same error; SyntaxShape::TypeAlias) |
| the rule "The "\<" ... ">" of a generic type ..." | src/winnow/syntax/parser/parse_shape_specs.rs::parse_generic_shape |
| `<shape-list>` | src/winnow/syntax/parser/parse_shape_specs.rs::parse_type_params |
| `<named-shapes>`, `<named-shape>`, `<field-name>` | src/winnow/syntax/parser/parse_shape_specs.rs::parse_named_type_params (the same skips; a non-item token or a non-string key such as "true" is "annotation key not string") |

### 7.3 Input/output types

| rules | winnow |
| --- | --- |
| `<io-types>`, `<io-pair>` | src/winnow/syntax/parser/parse_signatures.rs::parse_input_output_types |

### 7.4 Variable declarations with an optional type

| rules | winnow |
| --- | --- |
| `<var-decl>`, `<type-items>` | src/winnow/syntax/parser/parse_signatures.rs::parse_var_with_opt_type, parse_type_after_var, src/winnow/syntax/parser/parse_def.rs::parse_for, src/winnow/syntax/parser/parse_signatures.rs::ensure_not_reserved_variable_name (the reserved names) |

### 7.5 Import patterns

| rules | winnow |
| --- | --- |
| `<import-pattern-tail>`, `<member-name>`, `<glob>`, `<member-list>` | src/winnow/syntax/parser/parse_module.rs::parse_import_pattern_member (an ignored member is ImportPatternMemberKind::Ignored; dropped list items and cell paths are ignored text), parse_import_pattern_list |

### 7.6 Match patterns

| rules | winnow |
| --- | --- |
| `<pattern>` | src/winnow/syntax/parser/parse_patterns.rs::parse_pattern, pattern (the recovery) |
| `<variable-pattern>` | src/winnow/syntax/parser/parse_patterns.rs::parse_variable_pattern |
| `<variable-pattern>`, `<value-pattern>` | src/winnow/syntax/parser/parse_patterns.rs::parse_pattern (the constant check: consumer) |
| `<list-pattern>`, `<list-pattern-item>` | src/winnow/syntax/parser/parse_patterns.rs::parse_list_pattern over lite_parser::lite_parse_parts |
| `<record-pattern>`, `<record-pattern-item>` | src/winnow/syntax/parser/parse_patterns.rs::parse_record_pattern |

### 7.7 Row conditions

| rules | winnow |
| --- | --- |
| `<row-condition>` | src/winnow/syntax/parser/parse_source.rs::parse_where, src/winnow/syntax/parser/parse_expressions.rs::parse_row_condition, expand_row_condition, expand_to_cell_path (bool typing: consumer) |

## What a consumer adds

Every nonterminal of the grammar maps to code in this parser except the ones annotated
`consumer`. The table lists what a consumer with a command table must add to
get nu-parser's full behaviour; none of it is decidable from the text alone.

| shape-directed rule | what nu-parser needs | where the consumer hooks in |
| --- | --- | --- |
| positional argument shapes (`<positional>`) of ordinary commands | the command's signature | with a lookup, and for a command without a known signature, every `Argument::Positional` is a `ExpectedShape::Any` value; without one the built-ins' and the text's own definitions' are read by shape here; the keyword commands (6.5) and the built-in attributes are checked here |
| `--flag value` binding, unknown flags, short-flag batches, `-1` as flag or number, for ordinary commands | signature | `Argument::Named` (`equals` tells `--flag value` from `--flag=value`), `Argument::Positional`; bound here when the signature is known as above, unknown flags the consumer's; the keyword commands are checked here |
| spread validity (rest param, named flags) | signature | `Argument::Spread` |
| glob vs string vs filepath for bare words | signature | `Expr::String` with `Quote::Bare` |
| alias expansion and alias-subcommand heads | declarations | `Call` heads |
| `%name` must be a built-in; `^` required in an assignment rhs | built-in table | `ParseConfig` (both checked when a table is configured) |
| constant-ness of default values, completers, match value patterns, `use` heads, `source` paths; whether a completer's or attribute's command exists | const-eval, declarations | `Parameter::default`, `Parameter::completer`, `Pattern::Expression`, `Use::module`, `Attribute::name` |
| `where` bool typing, `try` handler types, column-name types, the type of an assignment or a typed `let` | types | `Where`, `Handler`, `Table`, `Assignment`, `Binding` |
| module, file, overlay and plugin resolution; variable existence | files, module table, scope | `Call` heads, `Var` |
| the commands a `use`d module brings into scope (`use std/log; log info`) | module resolution | `ParseConfig::add_commands`; without them such a head is an `ExternalCall` |

## 9. Verified differences between nu-parser and this parser: 5 open

The subsections keep the numbers they had in the grammar (9.1 to 9.4), which commit messages
and fixture comments cite.

Last verified on 2026-09-26 against nushell `7de074cfe` and `nu` 0.115.2; the
rows found on 2026-10-26 were verified against nushell `706259d6f` with the
harness.

| section | kind of difference | closed | open |
| --- | --- | --- | --- |
| 9.1 | nu-parser rejects, this parser accepts | 34 | **1** |
| 9.2 | nu-parser accepts, this parser rejects | 39 | **4** |
| 9.3 | both accept, but the parse differs | 11 | **0** |
| 9.4 | needs a signature, a declaration, const-eval, a type or a file | | not a grammar difference: the consumer's checks, listed below |

Of the 84 closed rows, 40 were closed on 2026-09-24, 31 on 2026-09-25 and
2026-09-26, 12 on 2026-10-26, and 1 on 2026-11-03. Each is pinned by a fixture under
`tests/winnow/fixtures/accept` or `tests/winnow/fixtures/reject`; the git log has the details.

Method: every input was run through `nu-check` on the PATH `nu` 0.115.2
(and, for the syntactic cases, through `nu-parser-check` built from nushell
`main` in `winnow/tools/nushell-harness`) and through `cargo run --example winnow_parse`.
The harness's nu-parser runs in the `nu` binary's engine (the commands of
nushell's `command_context.rs`, with its default features), and this parser's
command table is generated from that engine (`winnow/tools/scripts/gen-builtin-commands.nu`),
so both know the same commands. The harness keeps no list of tolerated
differences; every disagreement it finds is counted and printed. Over the
fixtures, the corpus, nu-std, nushell's own tests, nu_scripts, the command
examples and the book it finds no syntactic disagreement. The mutants (seed 1)
move whenever the inputs do. An error nu reports inside another file than the
input (a module file a `use` names) counts as needing a file (9.4): on a machine
with a plugin installed, a mutant `use ~/.cargo/bin/nu_plugin_query` makes nu parse
the binary. Two generated sets reach what the mutants do not (their inputs and
generators are in the corpus/generated directory of nushell/grammar): every keyword statement
with a help flag or a flag value in each slot, and the fixed-signature commands with
switch values and short-flag batches (`flag-values/gen_sweep.py`, 2,720 inputs; the
four rows left are the second open row of 9.2), and every one-line fixture and each
prefix of its words as an alias target (`alias-missing-positional/targets.json`,
7,172 inputs; its 14 disagreements are the alias rows of 9.1 and 9.2 and the `%`
alias target and a leading range of 9.4), and range items whose bound is a group or a raw string with text after
it, as a statement, an argument and a value (`range-bounds/gen.py`, 900 inputs; its
nine disagreements are the range rows of 9.1 and 9.2). Differences that nu reports
only through type checking, constant
evaluation, signatures or module resolution are in 9.4, not counted as grammar
gaps.

### 9.1 nu-parser rejects, this parser accepts: 1 open

An `if`, `match` or `try` alias target that nu rejects through the keyword's
signature (DIFF 1): `alias zz = if --help if` and `alias zz = if if --help`
(expected expression), `alias zz = match 1 --help :{}` (expected match-block),
`alias zz = try {} -- catch` and `alias zz = try {} --help catch` (missing argument
to `catch`), `alias zz = try {} --help x` and `alias zz = try {} -- catch {} --`
(expected keyword). nu parses an alias target with `parse_call`, which reads these
keywords by their signatures (`if`: a math-expression condition, a block, `else`
and a block or an expression), not with their statement parsers; this parser parses
a complete target with the statement parser and a target with a flag as an
ordinary call, so it misses the errors. Closing it needs the signatures' positional
allocation for these three keywords, the same work as the alias row of 9.2. Found
with the alias-target set; no real script is known to hit it.

### 9.2 nu-parser accepts, this parser rejects: 4 open

An alias target whose first error is a missing positional nested inside it
(`alias i = if if x y`, `alias i = if (if x) {}`, `alias i = if 1 + if x y`), or
that nu reads through the keyword's signature where this parser's statement parser
fails (`alias i = if else { }`: a condition `else` and a block, DIFF 1).
nu parses the target, and when the first of its errors is a
MissingPositional, anywhere in it, drops them all; here that is the inner
`if`'s missing block. This parser stops at its first error, which carries no
missing-positional kind (the "missing" errors are `expected ...` diagnostics
from about 25 places, some split differently from nu's positional
allocation),
and it has no node for a target that nu accepts but that does not parse.
Closing it needs both: an `ErrorKind::MissingPositional` at those places and
a garbage target in `Alias`, or the signatures' positional allocation of 9.1's
alias row, which would decide both. The top-level case is handled
(`parse_aliased_keyword`: `alias i = if x`). Found by mutation fuzzing and the
alias-target set; no real script is known to hit it.

An `alias` or `type` help call with an argument this parser cannot parse
(`type --help= t = int`, `alias x --help=`). nu parses the items of these two
statements as a call only to see whether it asks for help and drops that call's
errors; this parser parses the help call as any other and reports "expected value
after `=`". Found with the flag sweep; no real script is known to hit it.

An argument that is a range whose bound is a subexpression followed by a broken
cell path (`echo 5..(1)!`, `echo 5..(1).(2)`, `echo 1..(1)!..5`): nu's value parser
gives up on the range and reads the word as a string; this parser commits to the
range and reports the cell path. As a statement head both read an external command
(3.1). Found with the range-bound set; no real script is known to hit it.

A `match` call for help with arms (`match 1 --help { 1 => 2, _ => 3 }`): nu reads
`match`'s second positional with the `match_block` shape, arms included, and shows
the help; this parser reads a call for help's arguments as values, so the `{ ... }`
is a closure whose body `1 => 2, _ => 3` does not parse. Where the body happens to
parse (`match --help 1 { _ => 2 }`), the front end hands the statement to nu-parser
(a closure where `match_block` wants arms). `parse_match` has the arms when it
finds the `--help`; closing it needs a node for a match block as a call's argument.
Found with the help-call set; no real script is known to hit it, so it is left open.

### 9.3 Both accept, but the parse differs: 0 open

None known outside 9.4.

### 9.4 Differences that need signatures, declarations, const-eval or files (consumer by design)

All verified with 0.115.2; nu rejects and this parser accepts unless noted. The
keyword commands (6.5) and the built-in attributes have fixed signatures and are
checked here; everything below concerns ordinary commands or needs more than the
text. The mutation fuzzer (`differential --mutants 3 --seed N`) keeps finding
rows of these classes that nu-parser reports as syntax errors, because the
variant is syntactic although the decision needs more than the text: `use eggs
foo    foo` after inline `module spam { export const foo = 3 }` and `module eggs
{ export use spam foo }` (WrongImportPattern: `foo` is a constant, not a
submodule), and `2 | skip 0x[01 23 45 67]` (InvalidLiteral: `skip` takes an
`int`).

- Positional counts and shapes of ordinary commands: `str foo`, `collect 1`, `def f
  [x] {}; f 1 2`, `f 1.0` for an `int` parameter, `f true` / `f null` for a `string`
  one, `f 0x[ff]`, `f 1..2`, `f $.a` for `int`, `2b | last 0x[01 23 45]`. The
  other way round, `f 0xzz` for a `string` parameter is accepted by nu (a string
  shape never tries the int parser) and rejected here, and so are `f [a]]` for a
  `string` parameter and `ls []]*` (a glob): read as a list, as a value of any shape
  is, the word has a `]` that closes nothing (5.14).
- Type aliases: what a type name means. Any word an alias could have is a type alias
  here (`def f [x: Pointt]`); nu reports an unknown one as UnknownType, one used above
  its `type` statement as TypeAliasUsedBeforeDefinition, one inside its own
  definition as TypeAliasSelfReference, and `Point<int>` for an alias as
  TypeAliasWithParams (an unknown generic-looking name is the consumer's too). A name
  no alias has is still a type in nu when some command declares it as an input or
  output type: a custom value type such as `semver` or a plugin's `polars_dataframe`
  (`find_custom_type`).
- Negating a value that holds no number: `let s = 'a'; -$s` is OperatorUnsupportedType
  in nu, a negation here (5.1).
- Flags of ordinary commands: unknown long or short flags (`foo --unknown`, `ls
  -lx`, `ls ---all`, `ls --=a`), also those of an `extern --strict` command (6.3);
  `--flag` without its value; `--switch=5`; `--switch 1`; a non-last short flag
  taking a value.
- Negative numbers: `echo -1`, `echo -.5`, `echo -1kb`, `echo -..3`, `ls -1`, `f -1`
  for a `string`/`any` parameter are "doesn't have flag" in nu; `-<digit>` after an
  ordinary command is a number here. `f -1` for `int`/`number` is accepted by both,
  and after a keyword (`return -1`) it is an error in both.
- Spreads without a rest parameter or named flags; `...$x` with an undefined `$x`;
  any undefined variable.
- Constant evaluation: non-constant defaults (`[x = (random int)]`) and completers
  (`x: string@(1 + 1)`), non-constant match value patterns (`match 1 { (random int)
  => 1 }`), non-constant `use`/`source` arguments (`use $x`, `source $x`), `const x
  = ls`. A flag of `def` or `extern` whose value is not `true` or `false` (`def
  --wrapped=$c`, `--env=(1 == 1)`): nu evaluates it, and with it the checks that
  depend on `--wrapped` (`const c = true; def --wrapped=$c f [...r: int] {}` is
  nu's type error; here neither the rest-type check nor the reserved `--help` check
  runs).
- Existence: a completer's command (`x: string@"unknown cmd"`) and an attribute's
  `attr <name>` command (`@nonexistent 1`), both of which may come from a `use`d
  module.
- Type checks: `where 5`, `where [1]`, `where 1 + 1`, `where -- 1`; `let x: int =
  "a"`; `$x ++= [1]` on an int; io-type mismatches; `try {} catch (ls)`; a column
  name that is a variable of the wrong type; `ls | overlay list`; `module -- {}`;
  the value of a keyword command's flag (`overlay hide --keep-env=1 o`, `overlay
  hide -ke o`, where `o` is the value) and a record where a Boolean is wanted
  (`def f [] {} --help={a: 1}`).
- Module, file, overlay and plugin resolution: `module m` with no file, `use ''`,
  `use std a *` where `a` is not a submodule, `use eggs foo foo` where `foo` is not
  a module, `use -- x`, `hide std *`, `overlay hide zero`, `plugin use x` with no
  such plugin, `source x.nu` with no such file.
- The command table: whether a head is an external command (4), with what
  follows from it (`^` required in an assignment rhs, external arguments,
  `cmd [a].0` unclosed), and whether `%name` is a built-in are decided only when
  a table is configured (`ParseConfig`; `ParseConfig::new()` has the commands
  of a fresh `nu`).
- The commands a `use`d module brings into scope: after `use std/log` or
  `module m { export def f [] {} }; use m`, nu resolves `log info` and `m f`
  as calls; without them in the `ParseConfig` (`add_commands`), such a head
  is an `ExternalCall` here (both accept; the tree differs).
- Variable existence decides a leading range: nu reads `..$x` as a range when
  `$x` exists and as an external command when it does not; here it is always
  a range (`..$$ a` and `..$x# a` go the other way: nu accepts them as
  externals when the variable does not exist). The same decides an alias
  target: `alias zz = 0..$x.len` names an external command for nu when `$x` does
  not exist and is CantAliasExpression when it does; here it is always the
  expression. The same decides an env
  shorthand: `FOO=$1` alone is an external command for nu when `$1` does not
  exist, and a shorthand without a command here.
- Commands a `use`d module brings into scope, when nu checks their arguments by
  signature: `use std/log; log info "x"=`, `use std/assert; assert equal
  'a'"a"`. And import patterns that need the module's contents: `use eggs foo
  foo` after inline modules, `use std/math PI PI`, `use m "sub cmd" "sub cmd"`.
- A `use` of a file that is not a module: nu opens and parses it (`use
  ~/.cargo/bin/nu_plugin_query` reads a binary).
- The other way round, and so not something a consumer can add: a positional
  of an ordinary command whose signature wants a glob or a string takes any
  word in nu, so `ls 0b2` and `ls [a-d]:1` are accepted by nu and rejected
  here (`echo 0b2` and `echo [a-d]:1`, whose positional is `any`, are
  rejected by both), like `f 0xzz` for a `string` parameter above. The
  mutation fuzzer finds more of these for built-in shapes: `glob
  "[!cCbMs]*"#`, `glob "**/*.txt": --follow-symlinks`, `glob "[A-Z]*"..`,
  `bytes split 0x$[20]`, `bytes split 0x[$20]`,
  `run-external "echo"..`; and the int shape the other way (`skip`, `first`,
  `last`, `take`, `drop`, `chunks` with `0x[..]`, `bytes add --index 0x[AA
  BB]`, `bits shr 0x[4f f4]`, `bytes at ..-4 ..-4`), cell-path shapes (`get
  home!] path!`, `group-by foo?$`) and `into sqlite [[name]; ...]`. The same
  goes for custom commands of the file: `def f [x: int] {}; f "a"`, `f {a: 1}`
  where a string is wanted, an extra positional, and a positional before the
  rest of a wrapped command (`def --wrapped f [x = "a", ...r] {}; f 'x'$`, where
  nu parses `'x'$` with the shape of `x` and this parser as an external argument;
  in `[...r string]` the `string` is such a positional).
  `do {a: 1}` is the built-in form of this: a record where a closure is wanted.
- A `%` alias target with a computed name (`alias zz = %(1 + 1)`, `alias zz = %
  ('echo') x`): nu makes a dynamic `%` call a placeholder call to declaration 0,
  in `nu`'s engine `alias`, so `finish_alias` reports CantAliasKeyword for an alias
  of `alias`; this parser reads a dynamic call and accepts it, and the front end,
  which lowers it to the same placeholder call, reports the same error. `alias zz =
  %ls` is accepted by both.
- A record spread whose variable nu has typed as a record keeps the record's
  type (`let r = {b: 2}; if true { a: 1, ...$r }` is a type error in nu); here a
  spread of a variable is typed `any` (5.13 \<block>).

## Keeping this file true

* Re-verify a row of section 9 by writing the input to a file and running
  `open --raw f | nu-check` and `cargo run --example winnow_parse -- --summary f`.
* When nushell's nu-parser changes, `tests/winnow/traceability.rs` fails on new
  `SyntaxShape`/`FlatShape`/`TokenContents`/`ParseError` variants, keyword commands and
  `parse_*` functions (`src/winnow/docs/11-traceability.md`). Update the rule in
  nushell/grammar and its row here at the same time.
* A new difference goes into 9.1, 9.2 or 9.3 with a `DIFF n` row on its rule above, and its
  open count in the table at the top of section 9 goes up. When it is fixed it becomes a
  fixture under `tests/winnow/fixtures/{accept,reject}`, its entry and the row go away, the
  open count goes down and the closed count up, and the date is updated. The git log keeps the
  details, not this file.
