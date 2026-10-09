# 11 Traceability: nu-parser's grammar, item by item

Nushell has no formal grammar; the language is what `nu-parser` accepts.
This chapter maps every syntactic unit nu-parser defines onto the code,
fixtures and tests of this crate, so that "the parser is complete" is an
auditable claim rather than an impression. `tests/traceability.rs` reads
these tables: every fixture pattern must match at least one file under
`tests/fixtures/`, every backticked test name must exist, and, when a
Nushell checkout is available (`../nushell` or `NU_WINNOW_NUSHELL`), every
variant of `SyntaxShape`, `FlatShape`, `TokenContents` and `ParseError`,
every keyword command and every `pub fn parse_*` in `nu-parser` must appear
in the matching table. A new construct upstream therefore fails the test
until it is mapped here.

Fixture patterns are relative to `tests/fixtures/` and may use `*`. A cell
reading `n/a` means nu-parser needs declarations, signatures, types or
files for that item, which this parser leaves to the consumer (chapter 08).

In the *here* column, `file::function` is a function of
`src/parser/<file>.rs` (`lex::` is `src/lex.rs`), and plain names after it
in the same cell are in the same file. The parser files are named after
nu-parser's files and most functions after their nu-parser counterparts,
so a cell often repeats its row's name: nu's `parse_def` is
`parse_def::parse_def` here.

## SyntaxShape

Each variant is a parse rule nu-parser applies to an argument position. nu
hands the shape to `parse_value`; here `parse_expressions::parse_value`
takes an `ExpectedShape` for the positions where the shape changes how an
item reads, and a parameter's declared type is an `ast::SyntaxShape` (nu's
enum name), with which its default value is parsed
(`ExpectedShape::Declared`).

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `Any` | `parse_expressions::parse_value` with `ExpectedShape::Any` | `accept/calls/args-of-every-kind.nu` `accept/lists/mixed-values.nu` | `barewords_in_argument_position_are_strings` |
| `Binary` | `parse_literals::parse_binary` | `accept/literals/binary-*.nu` `reject/literals/binary-*.nu` | `binary_literals` `invalid_binary_literals` |
| `Block` | `parse_expressions::parse_block_body`, reached through `parse_keywords::parse_block_argument` (statement bodies; `BraceShape::ClosureParams` and `BraceShape::Record` are refused first) | `accept/if/*.nu` `accept/loops/*.nu` `reject/closures/*-in-block-position.nu` `reject/if/then-block-is-record.nu` | `closures_and_blocks` |
| `Boolean` | `parse_expressions::parse_any_value` (`true`/`false`) | `accept/literals/bool-true-false.nu` | `numbers` |
| `CellPath` | `parse_literals::parse_dollar_expr` (`$.a.b`), `parse_simple_cell_path` (a default declared `cell-path`) | `accept/cellpaths/literal-cell-path*.nu` | `cell_path_literals_and_heads` |
| `Closure` | `parse_expressions::parse_closure_expression`, `ExpectedShape::Closure` | `accept/closures/*.nu` `accept/try/*.nu` | `closures_and_blocks` `try_forms` |
| `DateTime` | `parse_literals::is_datetime` | `accept/literals/datetime-*.nu` | `units_and_datetimes` |
| `Directory` | bare string (`Quote::Bare`); n/a without a signature | `accept/strings/bare-word.nu` | `number_like_strings` |
| `Duration` | `parse_literals::parse_duration` | `accept/literals/duration-*.nu` `reject/literals/duration-*.nu` | `durations` `unit_suffix_with_a_bad_number_is_an_error` |
| `Error` | n/a (a runtime value) | `accept/try/catch.nu` | `try_forms` |
| `Expression` | `parse_expressions::parse_expression` | `accept/if/else-expression.nu` `accept/match/body-forms.nu` | `if_forms` `match_forms` |
| `ExternalArgument` | `parse_calls::parse_external_arg`, `parse_external_string`, `check_external_list_argument` (a `[` argument must end with `]`) | `accept/externals/arg-*.nu` `reject/externals/*.nu` | `external_call_string_args` `external_call_structured_args` |
| `Filepath` | bare string; n/a without a signature | `accept/strings/bare-word.nu` `accept/externals/relative-path-command.nu` | `relative_path_is_an_external_command` |
| `Filesize` | `parse_literals::parse_filesize` | `accept/literals/filesize-*.nu` `reject/literals/filesize-*.nu` | `filesizes` |
| `Float` | `parse_literals::parse_float` | `accept/literals/float-*.nu` | `floats` |
| `FullCellPath` | `parse_literals::parse_full_cell_path` | `accept/cellpaths/*.nu` `reject/variables/*.nu` | `variables_and_cell_paths` `cell_path_members` |
| `GlobPattern` | bare string; n/a without a signature | `accept/strings/bare-word.nu` `accept/externals/head-forms.nu` | `external_call_heads` |
| `Int` | `parse_literals::parse_int`, `radix_prefix` | `accept/literals/int-*.nu` `reject/literals/int-*.nu` | `ints` `radix_prefixed_words_must_be_ints` |
| `ImportPattern` | `parse_module::parse_use`, `parse_import_pattern_member`, `parse_import_pattern_list` (dropped members are `ImportPatternMemberKind::Ignored` or ignored spans) | `accept/modules/use-*.nu` `reject/modules/use-*.nu` | `use_forms` |
| `Keyword` | keyword arguments inside statements (`in`, `else`, `catch`, `=`), mostly matched with the token parsers `tokens::keyword` and `tokens_until` | `accept/loops/for-*.nu` `accept/if/else*.nu` `accept/try/*.nu` `reject/loops/for-missing-in.nu` | `loops_and_jumps` `if_forms` |
| `List` | `parse_expressions::parse_list_expression` over `lite_parser::lite_parse_parts` (nu's lite parse of the bracket: `\|` splits, `;` is refused, a redirection is dropped) | `accept/lists/*.nu` `reject/lists/*.nu` | `lists` |
| `MathExpression` | `parse_expressions::parse_math_expression` (winnow's `expression` with `infix_operator` and `fold_binary_op`) | `accept/operators/*.nu` `reject/operators/*.nu` | `precedence` `unknown_operators_get_help` |
| `MatchBlock` | `parse_expressions::parse_match_block_expression`; `parse_control_flow::parse_match` keeps a closure, record, variable or subexpression in that position as `Match::value_block` | `accept/match/*.nu` `accept/match/block-is-*.nu` `reject/match/*.nu` | `match_forms` |
| `Nothing` | `parse_expressions::parse_any_value` (`null`) | `accept/literals/null.nu` `reject/records/null-key.nu` | `numbers` |
| `Number` | `parse_literals::parse_number` (range bounds, `ExpectedShape::Number`) | `accept/ranges/float-bounds.nu` `accept/ranges/negative-bounds.nu` | `ranges` |
| `OneOf` | the `else` branch: block or expression (`parse_control_flow::parse_block_or_value`); a default declared `oneof<...>` goes through `parse_expressions::parse_oneof` | `accept/if/else-expression.nu` `accept/if/else-call-with-closure-arg.nu` | `if_forms` |
| `Operator` | `parse_expressions::parse_operator`, `Operator::from_spelling` | `accept/operators/*.nu` `reject/operators/*.nu` | `all_operators_parse` `unknown_operators_get_help` |
| `Range` | `parse_literals::parse_range`, `is_range_syntax` | `accept/ranges/*.nu` `reject/ranges/*.nu` | `ranges` `bad_ranges` |
| `Record` | `parse_expressions::parse_record` (the interior is read token by token with `lex::next_token`) | `accept/records/*.nu` `reject/records/*.nu` | `records` `bare_colons_in_records_are_refused` |
| `RowCondition` | `parse_source::parse_where`, `parse_expressions::parse_row_condition` (`expand_to_cell_path` makes a bare left operand a `$it` cell path) | `accept/where/*.nu` `reject/where/*.nu` | `where_row_conditions` `empty_braces_as_row_condition` |
| `Signature` | `parse_signatures::parse_signature`, `check_parameter_order` | `accept/signatures/*.nu` `reject/signatures/*.nu` `accept/def/*.nu` `reject/def/*.nu` | `def_forms` `signature_forms_from_nushell_tests` |
| `ExternalSignature` | `parse_def::parse_extern` (same signature parser) | `accept/extern/*.nu` `reject/def/extern-*.nu` | `extern_alias_module_use_export` |
| `String` | `parse_literals::parse_string`, `parse_string_literal` | `accept/strings/*.nu` `reject/strings/*.nu` | `strings_all_quote_styles` `string_values` `string_escape_errors` |
| `Table` | `parse_expressions::parse_table_expression` (row shape, column count and column-name kind checked as nu does) | `accept/tables/*.nu` `reject/tables/*.nu` | `tables` |
| `VarWithOptType` | `parse_signatures::parse_var_with_opt_type`, `parse_type_after_var`; `parse_def::parse_for` (the type runs up to `in`: `tokens_until("in")`) | `accept/bindings/let-typed.nu` `reject/bindings/let-*.nu` `accept/loops/for-typed-record-with-spaces.nu` `reject/loops/for-typed-then-help.nu` | `let_mut_const` `let_type_annotations` `let_type_without_colon_is_extra_tokens` |

## Keyword commands

Commands whose `command_type` is `Keyword` in `nu-cmd-lang`, plus the
statements nu-parser's `parse_builtin_commands` hands to a parser of their
own. Here `parse_expressions::parse_builtin_commands` dispatches them too.
The ones that are ordinary commands in nu (`if`, `match`, `while`, `loop`,
`try`, `return`, `break`, `continue`) are in `parse_control_flow.rs`, a file
nu-parser does not have. `parse_keywords::KeywordCall` and
`keyword_boundary` give the statements nu's handling of flags and `--help`.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `alias` | `parse_alias::parse_alias` | `accept/alias/*.nu` `reject/def/alias-*.nu` | `extern_alias_module_use_export` |
| `break` | `parse_control_flow::parse_break_or_continue` | `accept/loops/break-continue-in-nested-blocks.nu` `reject/loops/break-with-arg.nu` | `loops_and_jumps` |
| `collect` | an ordinary `Call` (nu marks it a keyword for `$in` handling only) | `accept/calls/closure-arg-without-pipes.nu` | `multiword_commands_and_flags` |
| `const` | `parse_bindings::parse_const` | `accept/bindings/const-forms.nu` `reject/bindings/const-*.nu` | `let_mut_const` |
| `continue` | `parse_control_flow::parse_break_or_continue` | `accept/loops/loop-with-continue.nu` `reject/loops/continue-with-arg.nu` | `loops_and_jumps` |
| `def` | `parse_def::parse_def` | `accept/def/*.nu` `reject/def/*.nu` | `def_forms` `declaration_errors_from_nushell_tests` |
| `export` | `parse_module::parse_export_in_block` | `accept/modules/export-forms.nu` `reject/modules/export-*.nu` | `extern_alias_module_use_export` |
| `export alias` | `parse_module::parse_export_in_block` + `parse_alias::parse_alias` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `export const` | `parse_module::parse_export_in_block` + `parse_bindings::parse_const` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `export def` | `parse_module::parse_export_in_block` + `parse_def::parse_def` | `accept/modules/export-forms.nu` `accept/attributes/before-export.nu` | `extern_alias_module_use_export` |
| `export extern` | `parse_module::parse_export_in_block` + `parse_def::parse_extern` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `export module` | `parse_module::parse_export_in_block` + `parse_module::parse_module` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `export use` | `parse_module::parse_export_in_block` + `parse_module::parse_use` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `export-env` | `parse_module::parse_export_env` | `accept/modules/export-env-multiline.nu` `reject/modules/export-env-*.nu` | `extern_alias_module_use_export` |
| `extern` | `parse_def::parse_extern` | `accept/extern/*.nu` `reject/def/extern-*.nu` | `extern_alias_module_use_export` |
| `for` | `parse_def::parse_for` | `accept/loops/for-*.nu` `reject/loops/for-*.nu` | `loops_and_jumps` |
| `hide` | an ordinary `Call` checked against its `KeywordSignature` (`parse_calls::check_call`); module resolution is semantic | `accept/modules/hide-and-source.nu` `reject/calls/hide-*.nu` `reject/pipelines/hide-*.nu` `reject/redirections/hide-redirected.nu` | `use_forms` |
| `hide-env` | an ordinary `Call` | `accept/calls/keyword-named-commands.nu` | `use_forms` |
| `if` | `parse_control_flow::parse_if` | `accept/if/*.nu` `reject/if/*.nu` | `if_forms` |
| `let` | `parse_bindings::parse_let` | `accept/bindings/let-*.nu` `reject/bindings/let-*.nu` | `let_mut_const` |
| `loop` | `parse_control_flow::parse_loop` | `accept/loops/loop-*.nu` `reject/loops/loop-*.nu` | `loops_and_jumps` |
| `match` | `parse_control_flow::parse_match`, `parse_patterns.rs` | `accept/match/*.nu` `reject/match/*.nu` | `match_forms` |
| `module` | `parse_module::parse_module` | `accept/modules/module-*.nu` `reject/modules/module-*.nu` | `extern_alias_module_use_export` |
| `mut` | `parse_bindings::parse_mut` | `accept/bindings/mut-forms.nu` `reject/bindings/mut-*.nu` | `let_mut_const` |
| `overlay` | an ordinary `Call`; a redirection on any `overlay ...` is refused before its arguments are read | `accept/modules/overlay.nu` `reject/redirections/overlay-*.nu` `reject/pipelines/overlay-after-pipe.nu` | `use_forms` |
| `overlay hide` | an ordinary `Call` (multi-word head) checked by `parse_calls::check_call` | `accept/modules/overlay.nu` `reject/calls/overlay-hide-*.nu` | `use_forms` |
| `overlay new` | an ordinary `Call` (multi-word head) checked by `parse_calls::check_call` | `accept/modules/overlay.nu` `reject/calls/overlay-new-*.nu` `reject/pipelines/overlay-new-before-pipe.nu` | `use_forms` |
| `overlay use` | an ordinary `Call` (multi-word head) checked by `parse_calls::check_call` (the `as NAME` keyword included) | `accept/modules/overlay.nu` `reject/calls/overlay-use-*.nu` | `use_forms` |
| `plugin use` | an ordinary `Call` (multi-word head) checked by `parse_calls::check_call` | `accept/modules/plugin.nu` `reject/calls/plugin-use-*.nu` `reject/pipelines/plugin-use-before-pipe.nu` | `use_forms` |
| `return` | `parse_control_flow::parse_return` | `accept/loops/return-forms.nu` `reject/loops/return-two-values.nu` | `loops_and_jumps` |
| `run` | an ordinary `Call` checked by `parse_calls::check_call` (unknown flags pass through); `run` is refused as a definition name | `reject/def/keyword-name-run.nu` `reject/calls/run-without-argument.nu` `reject/redirections/run-redirected.nu` | `definitions_cannot_use_parser_keywords` |
| `source` | an ordinary `Call` checked by `parse_calls::check_call`; file resolution is semantic | `accept/modules/hide-and-source.nu` `reject/calls/source-*.nu` `reject/pipelines/source-*.nu` `reject/redirections/source-redirected.nu` | `use_forms` |
| `source-env` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/hide-and-source.nu` | `use_forms` |
| `try` | `parse_control_flow::parse_try` | `accept/try/*.nu` `reject/try/*.nu` | `try_forms` |
| `use` | `parse_module::parse_use` | `accept/modules/use-*.nu` `reject/modules/use-*.nu` | `use_forms` |
| `where` | `parse_source::parse_where` | `accept/where/*.nu` `reject/where/*.nu` | `where_row_conditions` |
| `while` | `parse_control_flow::parse_while` | `accept/loops/while-*.nu` `reject/loops/while-*.nu` | `loops_and_jumps` |

## TokenContents

nu's lexer token kinds and the [`TokenContents`](crate::lex::TokenContents)
variants they map to. The enum and its plain variants have nu's names; nu's
eight redirection kinds are one `Redirection` variant here, carrying a
`RedirectionOperator`, the assignment operator carries its
`AssignmentOperator`, and the lexer ends every token list with an `Eof`
token, which nu does not have.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `Item` | `TokenContents::Item` | `accept/pipelines/single-call.nu` | `lex_parenthesised_expression_is_one_item` `lex_interpolation_is_one_item` |
| `Comment` | `TokenContents::Comment` | `accept/comments/*.nu` | `lex_comment_spans` `lex_hash_inside_brackets` |
| `Pipe` | `TokenContents::Pipe` | `accept/pipelines/*.nu` | `simple_call_and_pipeline` |
| `PipePipe` | `TokenContents::PipePipe` (an error outside closure parameters) | `accept/closures/empty-params-no-space.nu` `reject/lexer/or-or.nu` | `bashisms_are_reported_with_help` |
| `AssignmentOperator` | `TokenContents::AssignmentOperator(AssignmentOperator)` | `accept/assignments/*.nu` | `assignments` |
| `ErrGreaterPipe` | `TokenContents::Redirection(RedirectionOperator::ErrPipe)` | `accept/redirections/stderr-pipe.nu` | `redirections` |
| `OutErrGreaterPipe` | `TokenContents::Redirection(RedirectionOperator::OutErrPipe)` | `accept/redirections/both-pipe.nu` | `redirections` |
| `Semicolon` | `TokenContents::Semicolon` | `accept/pipelines/semicolon-separated.nu` | `statements_separated_by_semicolons_and_newlines` |
| `OutGreaterThan` | `TokenContents::Redirection(RedirectionOperator::Out)` | `accept/redirections/stdout.nu` | `redirections` |
| `OutGreaterGreaterThan` | `TokenContents::Redirection(RedirectionOperator::OutAppend)` | `accept/redirections/append.nu` | `redirections` |
| `ErrGreaterThan` | `TokenContents::Redirection(RedirectionOperator::Err)` | `accept/redirections/stderr.nu` | `redirections` |
| `ErrGreaterGreaterThan` | `TokenContents::Redirection(RedirectionOperator::ErrAppend)` | `accept/redirections/append.nu` | `redirections` |
| `OutErrGreaterThan` | `TokenContents::Redirection(RedirectionOperator::OutErr)` | `accept/redirections/both-streams.nu` | `redirections` |
| `OutErrGreaterGreaterThan` | `TokenContents::Redirection(RedirectionOperator::OutErrAppend)` | `accept/redirections/both-streams.nu` | `redirections` |
| `Eol` | `TokenContents::Eol` | `accept/pipelines/newline-separated.nu` `accept/pipelines/crlf-line-endings.nu` | `lex_newline_and_semicolon` |

## Lexer rules

The special characters nu's `lex.rs` reacts to, and where the same rule
lives in `src/lex.rs` (every name in this table is in that file). The lexer
reads the source as a character stream: `next_token` skips whitespace and
`lex_token` dispatches on the next character; `lex_item` measures an item
with `item_length`, a byte loop like the one in nu's `lex_item`, and
`item_contents` classifies it.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| whitespace (space, tab, `\r`) ends an item | `item_length` (its `is_terminator` closure, nu's `is_item_terminator`), `skip_whitespace` | `accept/pipelines/tabs-as-whitespace.nu` `accept/pipelines/crlf-line-endings.nu` | `lex_newline_and_semicolon` |
| `#` after whitespace starts a comment, not inside a word | `item_length` (`in_comment`, `previous`) | `accept/comments/comment-no-space.nu` `accept/comments/hash-inside-word-is-not-comment.nu` | `lex_comment_spans` `hash_without_preceding_space_is_not_a_comment` |
| a comment inside brackets runs to the end of the line | `item_length` (`in_comment` inside `brackets`) | `accept/comments/comment-in-list.nu` `reject/lexer/comment-swallows-closing-brace.nu` | `lex_hash_inside_brackets` |
| `"`, `'`, `` ` `` quotes, `\` escapes only in `"` | `item_length` (`quote`) | `accept/strings/*.nu` `reject/lexer/unclosed-*-quote*.nu` | `lex_unclosed_delimiters_point_at_the_opener` |
| `$"..("..")"` subexpression delimiters inside interpolation | `interp_subexpr_step` (nu's name) | `accept/interpolation/quotes-inside-subexpression.nu` `reject/lexer/unclosed-interpolation-*.nu` | `lex_interpolation_is_one_item` `unclosed_interpolation_subexpression_names_the_paren` |
| `r#'..'#` raw strings | `lex_raw_string` (nu's name) | `accept/strings/raw-string*.nu` `reject/lexer/unclosed-raw-string.nu` `reject/lexer/raw-string-missing-quote.nu` | `strings_all_quote_styles` |
| `(`, `[`, `{` nest; `)`, `]`, `}` must match; a stray `]` is text | `close_bracket`, `group_end` | `reject/lexer/mismatched-*.nu` `reject/lexer/unbalanced-*.nu` `reject/lexer/extra-*.nu` | `lex_mismatched_closers` `valid_layouts_never_get_delimiter_errors` |
| `<` and `>` nest inside signatures | `LexOptions::in_signature` | `accept/signatures/generic-types*.nu` `reject/def/io-unclosed-*.nu` | `lex_signatures` `lex_unterminated_type_annotations` |
| `\|` ends an item and is a pipe; `\|\|` is one token | `lex_token` | `accept/pipelines/double-pipe-tolerated.nu` `reject/lexer/or-or.nu` | `simple_call_and_pipeline` |
| `;` ends an item | `lex_token` | `accept/pipelines/semicolon-separated.nu` | `statements_separated_by_semicolons_and_newlines` |
| `o>`, `e>`, `o+e>` and `>>`, `>\|` variants are redirection tokens | `item_contents`, `is_redirection` (nu's name) | `accept/redirections/*.nu` `reject/redirections/*.nu` | `redirections` `redirecting_nothing_is_an_error` |
| `=`, `+=`, `-=`, `*=`, `/=`, `++=` are assignment tokens | `item_contents` | `accept/assignments/compound.nu` | `assignments` |
| `&&`, `2>`, `2>&1`, `o>\|` are refused with a hint | `item_contents` | `reject/lexer/and-and.nu` `reject/lexer/bash-*.nu` `reject/lexer/stdout-pipe-redirect.nu` | `bashisms_are_reported_with_help` |
| context-specific separators: `,` in lists, `:` in records, `.`/`?`/`!` in cell paths, `:`/`=`/`,` in signatures | `LexOptions` constants (`additional_whitespace`, `special_tokens`) | `accept/lists/mixed-separators.nu` `accept/records/no-space-after-colon.nu` `accept/cellpaths/optional-and-insensitive.nu` | `special_tokens_split` `lex_signatures` |

## FlatShape

nu's syntax-highlighting shapes and the [`FlatShape`](crate::flatten::FlatShape)
`flatten()` produces for the same text. `tools/scripts/flatcmp.nu` compares
the two over real files.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `Binary` | `FlatShape::Binary` | `accept/literals/binary-hex.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Block` | `FlatShape::Block` | `accept/if/simple.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Bool` | `FlatShape::Bool` | `accept/literals/bool-true-false.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Closure` | `FlatShape::Closure` | `accept/closures/one-param.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Custom` | n/a (plugin custom shapes) | `accept/modules/plugin.nu` | `shapes_cover_source_in_order_without_overlap` |
| `DateTime` | `FlatShape::DateTime` | `accept/literals/datetime-date.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Directory` | `FlatShape::String` (needs a signature) | `accept/strings/bare-word.nu` | `shapes_cover_source_in_order_without_overlap` |
| `External` | `FlatShape::External` | `accept/externals/caret-simple.nu` | `shapes_cover_source_in_order_without_overlap` |
| `ExternalArg` | `FlatShape::ExternalArg` | `accept/externals/caret-args.nu` | `shapes_cover_source_in_order_without_overlap` |
| `ExternalResolved` | n/a (nu-cli's highlighter marks an external found on `PATH`; nu-parser's flatten gives an unknown head `External`, as `flatten()` does) | `accept/externals/unknown-command-is-external.nu` | `relative_path_is_an_external_command` |
| `Filepath` | `FlatShape::String` (needs a signature) | `accept/strings/bare-word.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Flag` | `FlatShape::Flag` | `accept/calls/long-flags.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Float` | `FlatShape::Float` | `accept/literals/float-simple.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Garbage` | `FlatShape::Garbage` | `reject/operators/incomplete.nu` | `recovery_keeps_parsing_later_statements` |
| `GlobInterpolation` | `FlatShape::StringInterpolation` (needs a signature) | `accept/interpolation/bare-word-path.nu` | `interpolation_in_external_argument` |
| `GlobPattern` | `FlatShape::String` (needs a signature) | `accept/strings/bare-word.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Int` | `FlatShape::Int` | `accept/literals/int-decimal.nu` | `shapes_cover_source_in_order_without_overlap` |
| `InternalCall` | `FlatShape::InternalCall` | `accept/calls/no-args.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Keyword` | `FlatShape::Keyword` | `accept/bindings/let-simple.nu` | `shapes_cover_source_in_order_without_overlap` |
| `List` | `FlatShape::List` | `accept/lists/space-separated.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Literal` | `FlatShape::Filesize`, `FlatShape::Duration` (units) | `accept/literals/filesize-units.nu` | `shapes_cover_source_in_order_without_overlap` |
| `MatchPattern` | `FlatShape::MatchPattern` | `accept/match/list-patterns.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Nothing` | `FlatShape::Nothing` | `accept/literals/null.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Operator` | `FlatShape::Operator`, `FlatShape::Boolean` | `accept/operators/arithmetic.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Pipe` | `FlatShape::Pipe` | `accept/pipelines/two-elements.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Range` | `FlatShape::Range` | `accept/ranges/inclusive.nu` | `shapes_cover_source_in_order_without_overlap` |
| `RawString` | `FlatShape::String` (`Quote::Raw`) | `accept/strings/raw-string.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Record` | `FlatShape::Record` | `accept/records/basic.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Redirection` | `FlatShape::Redirection` | `accept/redirections/stdout.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Signature` | `FlatShape::Signature` | `accept/def/positional-params.nu` | `shapes_cover_source_in_order_without_overlap` |
| `String` | `FlatShape::String` | `accept/strings/double-quoted.nu` | `shapes_cover_source_in_order_without_overlap` |
| `StringInterpolation` | `FlatShape::StringInterpolation` | `accept/interpolation/double-basic.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Table` | `FlatShape::Table` | `accept/tables/basic.nu` | `shapes_cover_source_in_order_without_overlap` |
| `Variable` | `FlatShape::Variable` | `accept/variables/simple.nu` | `shapes_cover_source_in_order_without_overlap` |
| `VarDecl` | `FlatShape::VarDecl` | `accept/bindings/let-dollar-name.nu` | `shapes_cover_source_in_order_without_overlap` |

Shapes this crate has that nu does not: `Attribute`, `CellPath`,
`Comment`, `Definition`, `Ignored`, `Type`; `flatcmp.nu` maps them onto
nu's classes. Every other shape keeps nu's name, including `ExternalArg`
and `Redirection` (the AST nodes they mark are `ExternalArgument` and
`PipelineRedirection`).

## ParseError

Every variant of `nu_protocol::ParseError`. A *syntax* error must be
reproduced (with a `reject` fixture); a *semantic* one needs declarations,
signatures, types or files and is left to the consumer.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `ExtraTokens` | `ErrorKind::ExtraTokens` | `reject/bindings/let-missing-colon.nu` `reject/lexer/text-after-closing-quote.nu` | `let_type_without_colon_is_extra_tokens` |
| `ExtraTokensAfterClosingDelimiter` | `ErrorKind::ExtraTokens` | `reject/lexer/text-after-closing-brace.nu` | `malformed_input_does_not_panic` |
| `ExtraPositional` | `ErrorKind::Message` for the keyword commands with a fixed signature (`parse_calls::check_call` against a `KeywordSignature`); n/a for ordinary commands | `reject/calls/hide-extra-positional.nu` `reject/calls/overlay-new-two-positionals.nu` `accept/calls/positional-args.nu` | `multiword_commands_and_flags` |
| `RequiredAfterOptional` | `parse_signatures::check_parameter_order` | `reject/signatures/required-after-optional.nu` `accept/signatures/optional-positional.nu` | `def_forms` |
| `UnexpectedEof` | `ErrorKind::UnexpectedEof` | `reject/pipelines/pipe-then-semicolon.nu` `reject/pipelines/trailing-pipe-in-block.nu` | `redirecting_nothing_is_an_error` |
| `Unclosed` | `ErrorKind::Unclosed` | `reject/lexer/unclosed-*.nu` | `unclosed_delimiters_report_opener` `lex_unclosed_delimiters_point_at_the_opener` |
| `Unbalanced` | `ErrorKind::Unbalanced` | `reject/lexer/unbalanced-*.nu` `reject/lexer/mismatched-*.nu` | `lex_mismatched_closers` |
| `Expected` | `ErrorKind::Expected` | `reject/if/no-block.nu` `reject/match/no-arrow.nu` | `unknown_operator_hints` |
| `ExpectedWithStringMsg` | `ErrorKind::Expected`, `ErrorKind::Message` | `reject/def/missing-body.nu` | `declaration_errors_from_nushell_tests` |
| `ExpectedWithDidYouMean` | `ErrorKind::UnknownOperator` with help | `reject/operators/bits-and.nu` | `unknown_operators_get_help` |
| `InputMismatch` | n/a (semantic: types) | `accept/def/io-types-single.nu` | `def_forms` |
| `OutputMismatch` | n/a (semantic: types) | `accept/def/io-types-single.nu` | `def_forms` |
| `Mismatch` | `ErrorKind::Expected` (`expected block`) | `reject/closures/closure-params-in-block-position.nu` | `closures_and_blocks` |
| `ShellAndAnd` | `ErrorKind::ShellSyntax` | `reject/lexer/and-and.nu` | `bashisms_are_reported_with_help` |
| `ShellOrOr` | `ErrorKind::ShellSyntax` | `reject/lexer/or-or.nu` | `bashisms_are_reported_with_help` |
| `ShellErrRedirect` | `ErrorKind::ShellSyntax` | `reject/lexer/bash-stderr-redirect.nu` | `bashisms_are_reported_with_help` |
| `ShellOutErrRedirect` | `ErrorKind::ShellSyntax` | `reject/lexer/bash-both-redirect.nu` | `bashisms_are_reported_with_help` |
| `MultipleRedirections` | `ErrorKind::Message` (`multiple redirections`) | `reject/redirections/duplicate-*.nu` | `redirections` |
| `CaptureOfMutableVar` | n/a (semantic: scopes) | `accept/assignments/in-closure-and-block.nu` | `assignments` |
| `ExpectedKeyword` | `ErrorKind::ExpectedKeyword` | `reject/loops/for-missing-in.nu` | `loops_and_jumps` |
| `UnexpectedKeyword` | `ErrorKind::Expected` | `reject/if/else-before-condition.nu` | `if_forms` |
| `KeywordShadowModuleMain` | n/a (semantic: modules) | `accept/modules/module-with-main.nu` | `extern_alias_module_use_export` |
| `CantAliasKeyword` | `ErrorKind::Message` (parser keyword as alias name) | `reject/def/alias-keyword-name.nu` | `definitions_cannot_use_parser_keywords` |
| `CantAliasExpression` | `ErrorKind::Expected` (`command after =`) | `reject/def/alias-missing-target.nu` | `extern_alias_module_use_export` |
| `UnknownOperator` | `ErrorKind::UnknownOperator` | `reject/operators/*.nu` | `unknown_operators_get_help` |
| `BuiltinCommandInPipeline` | `parse_expressions::check_builtin_command_in_pipeline` (nu's head table for every element of a multi-command pipeline and after env shorthand; `ls \| let x` without a value is allowed, as in nu) | `reject/pipelines/def-in-pipeline.nu` `reject/pipelines/hide-after-pipe.nu` `accept/bindings/let-after-pipe.nu` | `let_after_pipe_is_a_statement` |
| `AssignInPipeline` | `parse_expressions::check_builtin_command_in_pipeline` (`ls \| let x = 1`, `ls \| mut x = 1`) | `reject/pipelines/let-with-value-after-pipe.nu` `reject/pipelines/mut-after-pipe.nu` `accept/assignments/rhs-pipeline.nu` | `assignments` |
| `NameIsBuiltinVar` | `parse_signatures::ensure_not_reserved_variable_name` (`in`, `nu`, `env`, `ans`; skipped for `extern`) | `reject/signatures/reserved-param-in.nu` `reject/signatures/reserved-flag-env.nu` `reject/match/variable-pattern-in.nu` `reject/loops/for-reserved-var.nu` `accept/def/reserved-param-names-in-extern.nu` | `reserved_variable_names_are_errors` `signature_forms_from_nushell_tests` |
| `NameIsKeyword` | `ErrorKind::Message` (`cannot use parser keyword`) | `reject/def/keyword-name-*.nu` | `definitions_cannot_use_parser_keywords` |
| `IncorrectValue` | n/a (semantic: constant evaluation) | `accept/attributes/values-of-every-kind.nu` | `attribute_values` |
| `InvalidBinaryString` | `ErrorKind::InvalidLiteral` (`binary`) | `reject/literals/binary-*.nu` | `invalid_binary_literals` |
| `MultipleRestParams` | `parse_signatures::check_parameter_order` | `reject/signatures/two-rest-params.nu` `accept/def/rest-param.nu` | `def_forms` |
| `VariableNotFound` | n/a (semantic: scopes) | `accept/variables/simple.nu` | `variables_and_cell_paths` |
| `EnvVarNotVar` | n/a (semantic) | `accept/env-shorthand/single.nu` | `env_shorthand` |
| `VariableNotValid` | `ErrorKind::Expected` (`valid variable name`) | `reject/bindings/let-quoted-name.nu` `reject/variables/name-with-dash.nu` | `declaration_errors_from_nushell_tests` |
| `AliasNotValid` | `ErrorKind::Expected` (`=`), `ErrorKind::Message` (a bare name starting with `-`, `#`, `^`, `%` or a number) | `reject/def/alias-missing-equals.nu` `reject/def/alias-name-with-hash.nu` `reject/alias/double-dash-name-alone.nu` | `extern_alias_module_use_export` |
| `CommandDefNotValid` | `ErrorKind::Expected` (`signature`) | `reject/def/missing-signature.nu` `reject/def/def-no-space-bracket.nu` | `def_forms` |
| `ModuleNotFound` | n/a (semantic: files) | `accept/modules/use-path.nu` | `use_forms` |
| `ModuleMissingModNuFile` | n/a (semantic: files) | `accept/modules/module-path.nu` | `extern_alias_module_use_export` |
| `CircularImport` | n/a (semantic: files) | `accept/modules/use-path.nu` | `use_forms` |
| `NamedAsModule` | n/a (semantic: modules) | `accept/modules/module-inline.nu` | `extern_alias_module_use_export` |
| `ModuleDoubleMain` | n/a (semantic: modules) | `accept/modules/module-with-main.nu` | `extern_alias_module_use_export` |
| `ExportMainAliasNotAllowed` | n/a (semantic: modules) | `accept/modules/module-inline.nu` | `extern_alias_module_use_export` |
| `ActiveOverlayNotFound` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `OverlayPrefixMismatch` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `ModuleOrOverlayNotFound` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `CantRemoveLastOverlay` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `CantHideDefaultOverlay` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `CantAddOverlayHelp` | n/a (semantic: overlays) | `accept/modules/overlay.nu` | `use_forms` |
| `DuplicateCommandDef` | `parse_def::parse_def_predecl` (the same `def`/`extern` name predeclared twice in one block); across modules and files: semantic | `reject/def/duplicate-*.nu` `accept/def/same-name-in-nested-block.nu` `accept/modules/module-inline.nu` | `extern_alias_module_use_export` |
| `UnknownCommand` | env shorthand without a command (`FOO=1` alone, `parse_expressions::parse_expression`); an unknown attribute name: consumer. An unknown head is not an error but an external call (`parse_calls::parse_call`) | `reject/env-shorthand/shorthand-alone.nu` `accept/externals/unknown-command-is-external.nu` `accept/attributes/unknown-attribute-name-is-consumer.nu` | `external_calls` `env_shorthand` |
| `UnknownFlag` | `ErrorKind::Message` (`doesn't have flag`) for the statements and the keyword commands (`parse_keywords::keyword_boundary`, `parse_calls::check_call`); n/a for ordinary commands | `reject/calls/hide-unknown-long-flag.nu` `reject/loops/return-unknown-flag.nu` `accept/calls/long-flags.nu` | `multiword_commands_and_flags` |
| `UnknownType` | `ErrorKind::UnknownType` | `reject/def/unknown-type.nu` `reject/bindings/let-unknown-type.nu` | `def_forms` |
| `MissingFlagParam` | `ErrorKind::Message` for the keyword commands (`parse_calls::check_call`); n/a for ordinary commands | `reject/calls/overlay-hide-keep-env-without-value.nu` `accept/calls/flag-followed-by-value.nu` | `multiword_commands_and_flags` |
| `OnlyLastFlagInBatchCanTakeArg` | n/a (semantic: signature) | `accept/calls/short-flag-batch.nu` | `multiword_commands_and_flags` |
| `MissingPositional` | `ErrorKind::Message` for the keyword commands (`parse_calls::check_call`); n/a for ordinary commands | `reject/calls/hide-without-argument.nu` `reject/calls/source-without-argument.nu` `accept/calls/no-args.nu` | `multiword_commands_and_flags` |
| `KeywordMissingArgument` | `ErrorKind::Expected` (`block`, `condition`), `parse_def::parse_for` ("missing argument to `in`": the last item is the block's, help or not) | `reject/if/no-block.nu` `reject/loops/while-no-block.nu` `reject/loops/for-help-then-in-without-value.nu` `reject/loops/for-help-then-in-without-block.nu` | `if_forms` |
| `MissingType` | `ErrorKind::Expected` (`type`) | `reject/def/type-missing.nu` `reject/def/io-missing-output.nu` | `def_forms` |
| `TypeMismatch` | n/a (semantic: types) | `accept/bindings/let-typed.nu` | `let_type_annotations` |
| `TypeMismatchHelp` | n/a (semantic: types) | `accept/bindings/let-typed.nu` | `let_type_annotations` |
| `MissingRequiredFlag` | n/a (semantic: signature) | `accept/calls/long-flags.nu` | `multiword_commands_and_flags` |
| `IncompleteMathExpression` | `ErrorKind::Expected` (`expression after operator`) | `reject/operators/incomplete.nu` | `unknown_operator_hints` |
| `UnknownState` | n/a (internal) | `accept/pipelines/empty.nu` | `empty_and_whitespace_only` |
| `InternalError` | n/a (internal) | `accept/pipelines/empty.nu` | `empty_and_whitespace_only` |
| `IncompleteParser` | n/a (internal) | `accept/pipelines/empty.nu` | `empty_and_whitespace_only` |
| `RestNeedsName` | `ErrorKind::Expected` (`valid variable name for this rest parameter`) | `reject/def/rest-param-with-dot.nu` | `def_forms` |
| `ParameterMismatchType` | `parse_expressions::parse_value_for_shape`, reached through `ExpectedShape::Declared` (a default parsed with the declared shape: `x: int = abc` is an error, `x: string = 1` is a string) | `reject/signatures/default-*.nu` `accept/def/param-defaults-of-every-kind.nu` | `def_forms` |
| `NonConstantDefaultValue` | n/a (semantic: constant evaluation) | `accept/signatures/defaults-with-types.nu` | `def_forms` |
| `ExtraColumns` | `parse_expressions::parse_table_expression` (a row longer than the header); the record-type form is semantic | `reject/tables/extra-columns.nu` `accept/bindings/let-typed.nu` | `tables` `let_type_annotations` |
| `MissingColumns` | `parse_expressions::parse_table_expression` (a row shorter than the header); the record-type form is semantic | `reject/tables/missing-columns.nu` `accept/bindings/let-typed.nu` | `tables` `let_type_annotations` |
| `AssignmentMismatch` | n/a (semantic: types) | `accept/assignments/simple.nu` | `assignments` |
| `WrongImportPattern` | `ErrorKind::Message` (`* or [...] member can only be at the end`, `wrong import pattern structure` for a member that is not a string); the "not a module" form is semantic | `reject/modules/use-glob-not-last.nu` `reject/modules/use-list-not-last.nu` `reject/modules/use-member-*.nu` | `use_forms` |
| `ExportNotFound` | n/a (semantic: modules) | `accept/modules/use-inline-module.nu` | `use_forms` |
| `SourcedFileNotFound` | n/a (semantic: files) | `accept/modules/hide-and-source.nu` | `use_forms` |
| `RegisteredFileNotFound` | n/a (semantic: files) | `accept/modules/plugin.nu` | `use_forms` |
| `FileNotFound` | n/a (semantic: files) | `accept/modules/const-and-source.nu` | `use_forms` |
| `InvalidLiteral` | `ErrorKind::InvalidLiteral`, `ErrorKind::Message` (record colons) | `reject/strings/*.nu` `reject/records/*-colon*.nu` | `string_escape_errors` `bare_colons_in_records_are_refused` |
| `LabeledError` | n/a (semantic: constant evaluation) | `accept/attributes/values-of-every-kind.nu` | `attribute_values` |
| `RedirectingBuiltinCommand` | `ErrorKind::Message` (`this statement cannot be redirected`; `parse_pipelines::rejects_redirection` covers the statements, every `overlay ...` and the keyword commands whose signature is not redirectable) | `reject/def/redirect-after-def.nu` `reject/redirections/*-redirected.nu` | `redirections` |
| `UnexpectedSpreadArg` | n/a (semantic: signature) | `accept/calls/spread-args.nu` | `multiword_commands_and_flags` |
| `AssignmentRequiresMutableVar` | n/a (semantic: scopes) | `accept/assignments/simple.nu` | `assignments` |
| `AssignmentRequiresVar` | `ErrorKind::Message` (`assignment requires a variable`) | `reject/assignments/to-literal.nu` `reject/assignments/to-call.nu` | `assignments` |
| `AttributeRequiresDefinition` | `ErrorKind::Expected`, `ErrorKind::Message` (attributes) | `reject/calls/attributes-*.nu` `reject/attributes/*.nu` | `attributes` |
| `UnexpectedRedirection` | `ErrorKind::Message` (`nothing to redirect`) | `reject/redirections/nothing-to-redirect*.nu` | `redirecting_nothing_is_an_error` |
| `OperatorUnsupportedType` | n/a (semantic: types) | `accept/operators/arithmetic.nu` | `precedence` |
| `OperatorIncompatibleTypes` | n/a (semantic: types) | `accept/operators/arithmetic.nu` | `precedence` |
| `NonUtf8` | n/a (the crate takes `&str`; invalid UTF-8 is refused by the caller) | `accept/misc/unicode-everywhere.nu` | `unicode_source` |
| `ScriptFileTooLarge` | n/a (file loading) | `accept/misc/config-record.nu` | `lex_large_nested_record_completes` |
| `ScriptFileNotText` | n/a (file loading) | `accept/misc/config-record.nu` | `lex_large_nested_record_completes` |
| `PluginNotFound` | n/a (semantic: plugin registry) | `accept/modules/plugin.nu` | `use_forms` |
| `LabeledErrorWithHelp` | `ErrorKind::Message` with help (`%` sigil, attributes) | `reject/calls/percent-sigil-*.nu` | `percent_sigil_requires_a_builtin` |

The `%` sigil's "percent sigil requires a built-in command" is a
`LabeledErrorWithHelp` in nu; here it is `ErrorKind::Message` with the same
text (`reject/calls/percent-sigil-*.nu`, `percent_sigil_requires_a_builtin`).

## Parser entry points

Every `pub fn parse_*` in `nu-parser`, and the function that plays its role
here. Rows marked n/a are stages this parser does not have (declaration
resolution, module files, type checking). Where the *here* cell starts with
the row's own name, the function has nu's name and sits in the file of the
same name as nu-parser's, except `parse_brace_expr` and
`parse_row_condition`: both are in `parse_expressions.rs` here, while nu
keeps them in `parse_literals.rs` and `parse_signatures.rs`. A function
that reads one item takes the item's span, like nu's
`parse_value(working_set, span, shape)`; one that reads a sequence of items
takes a `Tokens` stream (`src/parser/tokens.rs`) that winnow's combinators
drive.

| nu-parser | here | fixtures | tests |
| --- | --- | --- | --- |
| `parse_alias` | `parse_alias::parse_alias` (the target through `parse_calls::parse_call_lenient`, missing positionals forgiven as nu does) | `accept/alias/*.nu` `reject/alias/*.nu` | `extern_alias_module_use_export` |
| `parse_assignment_expression` | `parse_expressions::parse_assignment_expression` | `accept/assignments/*.nu` | `assignments` |
| `parse_assignment_operator` | `lex::item_contents` (`TokenContents::AssignmentOperator`) | `accept/assignments/compound.nu` | `assignments` |
| `parse_attribute` | `parse_calls::parse_attribute` | `accept/attributes/with-args.nu` | `attributes` |
| `parse_attribute_block` | `lite_parser::lite_attribute_lines`, `parse_pipelines::parse_pipeline_element` | `accept/attributes/*.nu` | `attributes` |
| `parse_binary` | `parse_literals::parse_binary`, `parse_binary_with_base` | `accept/literals/binary-hex.nu` | `binary_literals` |
| `parse_block` | `parse_pipelines::parse_block` (`lite_parser::last_non_comment_token` applies the trailing-pipe rule to the whole block) | `accept/pipelines/*.nu` `reject/pipelines/trailing-pipe-*.nu` | `statements_separated_by_semicolons_and_newlines` |
| `parse_block_expression` | `parse_expressions::parse_block_expression`, `parse_block_body` | `accept/match/body-forms.nu` | `match_forms` |
| `parse_brace_expr` | `parse_expressions::parse_brace_expr`, `brace_shape` (`BraceShape::{Empty, ClosureParams, Record, Spread, Other}`) | `accept/closures/*.nu` `accept/records/*.nu` | `closures_and_blocks` `records` |
| `parse_builtin_commands` | `parse_expressions::parse_builtin_commands` | `accept/misc/every-statement-in-closure.nu` | `if_forms` |
| `parse_call` | `parse_calls::parse_call` | `accept/calls/*.nu` | `multiword_commands_and_flags` |
| `parse_cell_path` | `parse_literals::parse_cell_path` (the combinator `path_member` over a `Tokens` stream) | `accept/cellpaths/*.nu` | `cell_path_members` |
| `parse_closure_expression` | `parse_expressions::parse_closure_expression`, `parse_closure_parts` | `accept/closures/*.nu` | `closures_and_blocks` |
| `parse_completer` | `parse_shape_specs::parse_completer`, called by `parse_shape_name` (a string or a list; whether the command exists is the consumer's) | `accept/def/completer.nu` `reject/signatures/completer-*.nu` | `def_forms` |
| `parse_const` | `parse_bindings::parse_const` | `accept/bindings/const-forms.nu` | `let_mut_const` |
| `parse_datetime` | `parse_literals::is_datetime` (the combinators `date`, `time`, `offset`) | `accept/literals/datetime-*.nu` | `units_and_datetimes` |
| `parse_def` | `parse_def::parse_def`, `parse_def_body`, `parse_signatures::parse_full_signature` (the body is parsed as a closure, as nu does) | `accept/def/*.nu` `reject/def/*.nu` | `def_forms` |
| `parse_def_predecl` | `parse_def::parse_def_predecl` (also reports a duplicate `def`/`extern` in one block) | `accept/calls/user-defined-later-in-file.nu` `reject/def/duplicate-*.nu` | `user_defined_multiword_commands_resolve` |
| `parse_directory` | bare string (n/a) | `accept/strings/bare-word.nu` | `number_like_strings` |
| `parse_duration` | `parse_literals::parse_duration` | `accept/literals/duration-units.nu` | `durations` |
| `parse_export_env` | `parse_module::parse_export_env` | `accept/modules/export-env-multiline.nu` | `extern_alias_module_use_export` |
| `parse_export_in_block` | `parse_module::parse_export_in_block` | `accept/modules/export-forms.nu` | `extern_alias_module_use_export` |
| `parse_export_in_module` | `parse_module::parse_export_in_block` (a module body is parsed as a block) | `accept/modules/module-inline.nu` | `extern_alias_module_use_export` |
| `parse_expression` | `parse_expressions::parse_expression` | `accept/pipelines/*.nu` | `simple_call_and_pipeline` |
| `parse_extern` | `parse_def::parse_extern`, `parse_signatures::parse_full_signature` (the signature takes every remaining item, so a body is dropped as nu drops it) | `accept/extern/*.nu` | `extern_alias_module_use_export` |
| `parse_external_call` | `parse_calls::parse_external_call` | `accept/externals/*.nu` | `external_calls` `external_call_heads` |
| `parse_filepath` | bare string (n/a) | `accept/strings/bare-word.nu` | `number_like_strings` |
| `parse_filesize` | `parse_literals::parse_filesize` | `accept/literals/filesize-units.nu` | `filesizes` |
| `parse_float` | `parse_literals::parse_float` | `accept/literals/float-simple.nu` | `floats` |
| `parse_for` | `parse_def::parse_for` | `accept/loops/for-list.nu` | `loops_and_jumps` |
| `parse_fresh` | n/a (re-parse for completions) | `accept/pipelines/empty.nu` | `empty_and_whitespace_only` |
| `parse_full_cell_path` | `parse_literals::parse_full_cell_path` | `accept/cellpaths/*.nu` | `variables_and_cell_paths` |
| `parse_full_signature` | `parse_signatures::parse_full_signature` (one item; two items with a `{` second, which is dropped; the colon forms) | `accept/def/io-types-list.nu` `accept/def/block-before-body-ignored.nu` `reject/def/io-types-without-colon.nu` | `def_forms` |
| `parse_glob_pattern` | bare string (n/a) | `accept/strings/bare-word.nu` | `external_call_heads` |
| `parse_hide` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/hide-and-source.nu` `reject/calls/hide-*.nu` | `use_forms` |
| `parse_import_pattern` | `parse_module::parse_use`, `parse_import_pattern_member`, `parse_import_pattern_list` | `accept/modules/use-list-forms.nu` `reject/modules/use-*.nu` | `use_forms` |
| `parse_input_output_types` | `parse_signatures::parse_input_output_types` (`repeat_to_end(input_output_type)`) | `accept/def/io-types-*.nu` | `def_forms` |
| `parse_int` | `parse_literals::parse_int` | `accept/literals/int-*.nu` | `ints` |
| `parse_internal_call` | `parse_calls::parse_call`, `parse_call_arguments`, `check_call` for the keyword commands; `parse_keywords::KeywordCall` for the flags at a statement's positional boundaries | `accept/calls/*.nu` `reject/calls/*.nu` | `multiword_commands_and_flags` |
| `parse_keyword` | `parse_expressions::parse_builtin_commands`; `parse_keywords::KeywordCall` gives the statements nu's handling of flags and `--help` | `accept/misc/every-statement-in-closure.nu` | `if_forms` |
| `parse_let` | `parse_bindings::parse_let` | `accept/bindings/let-*.nu` | `let_mut_const` |
| `parse_list_expression` | `parse_expressions::parse_list_expression`, `lite_parser::lite_parse_parts` | `accept/lists/*.nu` `reject/lists/*.nu` | `lists` |
| `parse_list_pattern` | `parse_patterns::parse_list_pattern` over `lite_parser::lite_parse_parts` (the rest may stand anywhere, once) | `accept/match/list-pattern*.nu` | `match_forms` |
| `parse_match_block_expression` | `parse_expressions::parse_match_block_expression` (`repeat_till(0.., parse_match_arm, eof)`); `parse_control_flow::parse_match` (`Match::value_block`) | `accept/match/*.nu` | `match_forms` |
| `parse_math_expression` | `parse_expressions::parse_math_expression` | `accept/operators/*.nu` | `precedence` |
| `parse_module` | `parse_module::parse_module` | `accept/modules/module-*.nu` | `extern_alias_module_use_export` |
| `parse_module_block` | `parse_module::parse_module` (the body through `parse_expressions::parse_block_body`, in a new scope) | `accept/modules/module-inline.nu` | `extern_alias_module_use_export` |
| `parse_module_file_or_dir` | n/a (files) | `accept/modules/module-path.nu` | `extern_alias_module_use_export` |
| `parse_multispan_value` | the keyword arguments of the statements, read with `KeywordCall` and token parsers such as `tokens_until("else")` and `opt(keyword("else"))` (`parse_control_flow.rs`, `parse_def.rs`) | `accept/if/else-if-chain.nu` | `if_forms` |
| `parse_mut` | `parse_bindings::parse_mut` | `accept/bindings/mut-forms.nu` | `let_mut_const` |
| `parse_number` | `parse_literals::parse_number` | `accept/ranges/float-bounds.nu` | `ranges` |
| `parse_operator` | `parse_expressions::parse_operator` | `accept/operators/*.nu` | `all_operators_parse` |
| `parse_overlay_hide` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/overlay.nu` `reject/calls/overlay-hide-*.nu` | `use_forms` |
| `parse_overlay_new` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/overlay.nu` `reject/calls/overlay-new-*.nu` | `use_forms` |
| `parse_overlay_use` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/overlay.nu` `reject/calls/overlay-use-*.nu` | `use_forms` |
| `parse_paren_expr` | `parse_literals::parse_paren_expr`, `parse_expressions::parse_subexpression` | `accept/subexpressions/*.nu` | `subexpressions_span_lines` |
| `parse_pattern` | `parse_patterns::parse_pattern` | `accept/match/value-patterns.nu` | `match_forms` |
| `parse_pipeline` | `parse_pipelines::parse_pipeline`, `lite_parser::after_pipe`, `pipe_on_later_line` (`Eol (Comment Eol)* Pipe` continues a line; a blank line closes the pipeline) | `accept/pipelines/*.nu` `reject/pipelines/*.nu` | `multiline_pipelines_with_leading_pipes_and_comments` |
| `parse_plugin_use` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/plugin.nu` `reject/calls/plugin-use-*.nu` | `use_forms` |
| `parse_range` | `parse_literals::parse_range` | `accept/ranges/*.nu` | `ranges` |
| `parse_raw_string` | `parse_literals::parse_raw_string` | `accept/strings/raw-string.nu` | `strings_all_quote_styles` |
| `parse_record` | `parse_expressions::parse_record` | `accept/records/*.nu` | `records` |
| `parse_record_pattern` | `parse_patterns::parse_record_pattern` (every token is a field token, kept verbatim) | `accept/match/record-pattern*.nu` `reject/match/record-pattern-*.nu` | `match_forms` |
| `parse_row_condition` | `parse_expressions::parse_row_condition` | `accept/where/*.nu` | `where_row_conditions` |
| `parse_run` | an ordinary `Call` checked by `parse_calls::check_call` | `reject/def/keyword-name-run.nu` `reject/calls/run-without-argument.nu` | `definitions_cannot_use_parser_keywords` |
| `parse_run_expr` | an ordinary `Call` checked by `parse_calls::check_call` | `reject/def/keyword-name-run.nu` | `definitions_cannot_use_parser_keywords` |
| `parse_shape_name` | `parse_shape_specs::parse_type` (the table of shape names), called by `parse_shape_name`, which also splits off a `@completer` | `accept/signatures/all-types.nu` | `types_in_signatures` |
| `parse_shorter_head_reading` | `parse_calls::find_longest_decl` (longest known name) | `accept/calls/multiword-command.nu` | `multiword_commands_and_flags` |
| `parse_signature` | `parse_signatures::parse_signature` | `accept/signatures/*.nu` | `def_forms` |
| `parse_signature_helper` | `parse_signatures::parse_signature_helper`, `parse_parameters` (nu's `ParseMode` state machine) | `accept/signatures/*.nu` | `def_forms` |
| `parse_simple_cell_path` | `parse_literals::parse_simple_cell_path`, `parse_dollar_expr` (`$.a`) | `accept/cellpaths/literal-cell-path.nu` | `cell_path_literals_and_heads` |
| `parse_source` | an ordinary `Call` checked by `parse_calls::check_call` | `accept/modules/hide-and-source.nu` `reject/calls/source-*.nu` | `use_forms` |
| `parse_string` | `parse_literals::parse_string` | `accept/strings/*.nu` | `string_values` |
| `parse_string_interpolation` | `parse_literals::parse_string_interpolation`, `parse_interpolation_parts` | `accept/interpolation/*.nu` | `interpolation_parts` |
| `parse_string_strict` | `parse_literals::parse_string_literal` (env shorthand values) | `accept/env-shorthand/quoted-value.nu` | `env_shorthand` |
| `parse_type` | `parse_shape_specs::parse_type` | `accept/signatures/generic-types.nu` | `types_in_signatures` |
| `parse_unit_value` | `parse_literals::parse_unit_value` | `accept/literals/filesize-units.nu` | `filesizes` `durations` |
| `parse_use` | `parse_module::parse_use` | `accept/modules/use-std.nu` | `use_forms` |
| `parse_value` | `parse_expressions::parse_value` | `accept/calls/args-of-every-kind.nu` | `barewords_in_argument_position_are_strings` |
| `parse_var_with_opt_type` | `parse_signatures::parse_var_with_opt_type` | `accept/bindings/let-typed.nu` | `let_mut_const` |
| `parse_variable_expr` | `parse_literals::parse_variable_expr` | `accept/variables/*.nu` | `variables_and_cell_paths` |
| `parse_variable_pattern` | `parse_patterns::parse_variable_pattern` | `accept/match/all-pattern-kinds.nu` | `match_forms` |
| `parse_where` | `parse_source::parse_where` | `accept/where/*.nu` | `where_row_conditions` |
| `parse_where_expr` | `parse_source::parse_where` | `accept/where/closure.nu` | `where_row_conditions` |
