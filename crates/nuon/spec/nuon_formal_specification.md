# Nuon Formal Specification

- TODO: this documents the desired behavior. delete this comment, and bugs_to_fix.md, when all the
  bugs are fixed. [bugs_to_fix](./bugs_to_fix.md)

- contents:
    - [value model](#value-model) - the types, and why a record is ordered
    - [text](#text) - encoding, NUL, bom
    - [grammar](#grammar) - the whole of it, in fifteen lines
    - [token boundaries](#token-boundaries) - the rule everything else hangs off
    - [bare words and quoting](#bare-words) - when a string may go unquoted, and the digit rule
    - [strings](#strings) - four forms, and the escape set
    - [numbers](#numbers) - integers, radix, floats, non-finite, overflow
    - [durations and filesizes](#durations-and-filesizes) - unit suffixes
    - [datetimes](#datetimes) - the only literal containing `:`
    - [binary, cell-paths and ranges](#binary-cell-paths-and-ranges) - the three types json has
       no equivalent for
    - [lists and records](#lists-and-records) - separators, duplicate keys, key order
    - [table form](#table-form) - the one thing json does not have
    - [output](#output) - the four styles, and column alignment
    - [resource limits](#resource-limits) - for implementers
    - [reference tables](#reference-tables) - every duration and filesize suffix, and the
       quote-forcing set
    - [conformance](#conformance) - checklist, and the questions this could not answer

- conventions in this file:
    - `nushell` blocks are commands with their real output.
    - `nuon` blocks are document text.
    - "reader" is `from nuon`; "writer" is `to nuon`.

- what nuon has that json does not:
    - comments, `#` to end of line
    - commas optional; separate with whitespace instead
    - trailing commas
    - unquoted strings
    - single-quoted, backtick-quoted and raw (`r#'...'#`) strings
    - hex, octal and binary integers; `_` digit separators; leading `+`; leading/trailing `.`
    - `nan`, `inf`, `-infinity`
    - duration (`2min`), filesize (`1kb`) and datetime (`2000-01-01T00:00:00+00:00`) literals
    - the **table** form: `[[name, age]; [Alice, 30], [Bob, 25]]`

- and what it takes away:
    - duplicate record keys are an error, not last-write-wins
    - record key order is part of the value
    - no `\uXXXX` - nuon spells it `\u{41}`
    - `binary` (`0x[be ef]`), `cell-path` (`$.a.b`) and `range` (`1..5`) have literal syntax
    - `closure`, `block`, `glob` and `error` have no nuon syntax

## value model

- value model:
    - `nothing` `null`; `bool` `true`/`false`; `int` signed 64-bit; `float` ieee-754 binary64
    - `string`; `duration` (nanoseconds); `filesize` (bytes); `datetime` (instant + utc offset)
    - `binary` a byte sequence; `cell-path` a path into a value; `range` a bounded sequence
    - `list`; `record` - an **ordered** sequence of key/value pairs. an implementation whose
       record type is a hash map cannot encode nuon correctly.
    - `table` is not a distinct type; it is a surface form denoting a list of records.
    - all three of `binary`, `cell-path` and `range` round-trip through the writer and reader,
       so an implementation that cannot represent them cannot read everything `to nuon` emits.
        ```nushell
        "[0x[be ef], $.a.b, 1..5]" | from nuon | to nuon                         # => [0x[BEEF], $.a.b, 1..5]
        ```

## text

- text:
    - input must be well-formed utf-8; ill-formed bytes are rejected, not repaired.
    - a raw NUL in a string body, bare word or key should be rejected. an explicit `\u{0}` or
       `\0` is a stated intent and is accepted.
    - a leading utf-8 bom is stripped, exactly one and only at the start. `U+FEFF` anywhere else
       is ordinary content. windows editors and powershell's `Out-File` both write one, so a
       reader that refuses them cannot open files people actually have.

## grammar

- grammar:
    ```
    document   := ws* doc_value ws*
    doc_value  := value except a top-level bare word
    value      := null | bool | number | duration | filesize | datetime
                | binary | cell_path | range
                | string | list | record | table
    list       := "[" (value (sep value)* sep?)? "]"
    record     := "{" (key ":" value (sep key ":" value)* sep?)? "}"
    table      := "[" list ";" (list (sep list)* sep?)? "]"
    binary     := "0x[" (hex | sep)* "]" | "0b[" (bit | sep)* "]"
    cell_path  := "$" ("." member)*
    member     := (bare | '"' esc* '"' | "'" any* "'" | int) "?"?
    range      := bound? ".." "<"? bound? | bound ".." bound ".." bound
    bound      := number
    key        := string
    string     := bare | '"' esc* '"' | "'" any* "'" | "`" any* "`" | raw
    raw        := "r" "#"{n>=1} "'" any* "'" "#"{n}
    sep        := "," | ws+
    ws         := " " | "\t" | "\r" | "\n" | comment
    comment    := "#" (not "\n")*        -- only at a token boundary
    bare       := (not ws and not one of ", : [ ] { } ;")+
    ```
    - a datetime must be matched before `bare`, because it is the one literal containing `:`.
    - `doc_value` exists because a bare word is the one value that is not a valid document on its
       own. see [bare words](#bare-words).
    - the empty document is an error. an empty byte sequence is not a value, and a reader that
       returns `null` for it leaves the caller unable to tell an empty file from a file
       containing `null`.

## token boundaries

- token boundaries - the single lexical rule that everything else hangs off. a boundary is the
   start of input, whitespace, or one of `, : [ ] { } ;`.
    - `#` opens a comment **only** at a boundary. glued to a preceding word it is an ordinary
       byte.
        ```nushell
        "[a#b]"           | from nuon | to json -r                               # => ["a#b"]
        "[a, # note\n b]" | from nuon | to json -r                               # => ["a","b"]
        ```
        - getting this wrong in either direction breaks documents. if you always treat `#` as a
           comment, `[a#b]` becomes an unterminated document. if you never do, every commented
           document breaks.
    - a quote character only opens a string at a boundary, for the same reason.
        ```nushell
        "['a' b]"  | from nuon | to json -r                                      # => ["a","b"]
        "[r'a' b]" | from nuon | to json -r                                      # => ["r'a'","b"]
        ```

## bare words

- bare words: a string written without quotes.
    - a bare word ends at whitespace or at one of `, : [ ] { } ;`. every other byte is legal
       inside one, including non-ascii.
        ```nushell
        "[a-b, -, --, a/b, a\\b, &, %, @, ^, ~, +, *, <, >, café]" | from nuon | to json -r
        # => ["a-b","-","--","a/b","a\\b","&","%","@","^","~","+","*","<",">","café"]
        ```
    - a bare word is **not a valid document on its own**, though it is valid nested. this is the
       one place a bare word is not interchangeable with its quoted form, and it is easy to miss
       because every other scalar - quoted strings, numbers, durations, datetimes, `inf` - is a
       perfectly good top-level document.
        ```nushell
        "abc"      | from nuon                 # => error
        "[abc]"    | from nuon | describe      # => list<string>
        "{k: abc}" | from nuon | describe      # => record<k: string>
        ```

- when the writer must quote: a string is emitted bare only if all of
    - it is non-empty, and
    - it contains no byte from the quote-forcing set, and
    - it contains no unicode decimal digit (general category `Nd`), and
    - it would not read back as some other type.
    - the quote-forcing set is `space tab cr lf ! " # $ ' ( ) , . : ; = ? ` [ ] { | }` and
       nothing else. `-`, `/`, `\`, `_` and every non-ascii byte are absent from it. full set in
       [reference tables](#reference-tables).

- one decimal digit anywhere in a string, in any position, means it cannot be written bare. that
   is the entire rule.
    ```nushell
    ["a1" "0x" abc "x-1" "a-b" "a_b" "1s" "E5" "3/4"] | to nuon
    # => ["a1", "0x", abc, "x-1", a-b, a_b, "1s", "E5", "3/4"]
    ```
    - it is a **rune** test against category `Nd`, not an ascii byte test.
        ```nushell
        ["a０" "a٣" "a½" "Ⅴ" "௰"] | to nuon                                    # => ["a０", "a٣", a½, Ⅴ, ௰]
        ```
        - `a０` U+FF10 fullwidth zero and `a٣` U+0663 arabic-indic three are quoted; `a½`, `Ⅴ`
           and `௰` (category `No`/`Nl`) are not. widening the rule to "any numeric character"
           over-quotes them - harmless to the decoded value, but a divergence.

- re-parse hazard: the quoting test exists so a bare word reads back as the string it was written
   as. quote any string whose text would decode as a non-string, at least:
    - `true`, `false`, `null`
    - anything the number grammar accepts, including `1_000`, `007`, `+5`, `.5`, `1.`
    - any non-finite spelling: `nan`, `NAN`, `Inf`, `INFINITY`, `-infinity`, `+nan`
    - any duration or filesize literal: `2min`, `1kb`, `1_000ns`, `1μs`
    - any datetime literal: `2000-01-01`, `2000-01-01T12:34:56+05:30`
    - any token that makes the reader **fail** rather than misread.

- committed prefixes make this worse than a type change. having seen `0x`, `0o` or `0b` the reader
   commits to a radix literal and errors if one does not follow, so the ordinary strings `0x`,
   `0o8`, `0b2`, `0xg` written bare produce documents nushell **refuses to open**.
    ```nushell
    "[0o8]" | from nuon                                                         # => error: Invalid literal
    ```
    - the same goes for unit spellings nushell knows but whose value overflows, or whose form is
       just close enough: `1zb`, `1yb`, `1zib`, `1yib`, `1mins`, `1ab` and `9999999999999wk` are
       hard errors, while `1foo` is the string `"1foo"`. the digit rule already covers all of
       them, which is why it is easier to implement than a list of special cases.

## strings

- strings - three quoted forms plus the bare form:
    - `"..."` double quoted, escapes processed.
    - `'...'` single quoted, **literal**: no escape processing.
    - `` `...` `` backtick quoted, literal, same as single quoted.
        ```nushell
        "['a\\tb', `a b`]" | from nuon | to json -r                              # => ["a\\tb","a b"]
        ```
    - `r#'...'#` raw string. the hash run may be any length >= 1; the terminator is `'` followed
       by the same number of hashes. the writer grows the run for **two** reasons: until the
       terminator does not occur in the body, and until the run is longer than any hash run the
       body starts with. so any content is representable.
        ```nushell
        'hello "world"' | to nuon --raw-strings                                  # => r#'hello "world"'#
        "[r##'it'#s'##]" | from nuon | to json -r                                # => ["it'#s"]
        ```
        - the second reason is easy to miss, because the body's leading hashes are nowhere near
           the terminator. a writer that only checks for the terminator emits `r#'#"foo'#`, which
           does not read back as intended.
            ```nushell
            '#"foo'  | to nuon --raw-strings                                     # => r##'#"foo'##
            '##"foo' | to nuon --raw-strings                                     # => r###'##"foo'###
            ```
        - at least one hash is required. `r'a'` is not a raw string and is not an error either -
           `'` is not a bare-word terminator, so it is the single bare word `r'a'`.
            ```nushell
            "[r'a']" | from nuon | to json -r                                    # => ["r'a'"]
            ```

- escapes valid inside `"..."`, twenty-one single-character forms plus two with arguments:
    - `\"` `\'` `\\` `\/` - the literal character
    - `\(` `\)` `\{` `\}` `\$` `\^` `\#` `\|` `\~` - also the literal character. these exist
       because nushell gives those bytes meaning elsewhere, and escaping them is harmless here.
    - `\n` `\t` `\r` `\b` `\f` `\a` `\e` - control characters. `\a` is U+0007, `\e` is U+001B.
    - `\0` - U+0000
    - `\xHH` - a **byte**, not a character. the bytes an escape run produces must together form
       valid utf-8, so `\xC3\xA9` is `é` while `\x80` and `\xff` alone are errors.
        ```nushell
        "[\"\\xC3\\xA9\", \"\\x41\"]" | from nuon | to json -r                   # => ["é","A"]
        "[\"\\xff\"]" | from nuon                                                # => error
        ```
    - `\u{H...}` - one to six hex digits, max `0x10FFFF`. surrogates `D800`-`DFFF` rejected. the
       digit count is capped at six even for small values, so `\u{0000041}` is an error.
    - an implementation that stops at the json set will reject valid nuon. the nine bracket and
       sigil escapes are the ones most likely to be missed.
        ```nushell
        "[\"a\\(b\", \"a\\#b\", \"a\\~b\", \"a\\ab\"]" | from nuon | to json -r  # => ["a(b","a#b","a~b","a\u0007b"]
        ```

- escapes that are **not** nuon, despite being json or json5:
    - `\uXXXX` - nuon requires the braces, and consequently has no utf-16 surrogate pairing.
    - `\v` - ECMAScript's vertical tab.
    - `` \` `` - there is never a backtick to escape inside a double-quoted string.
    - a backslash-newline line continuation.
    ```nushell
    '["a\u0041b"]' | from nuon                                                  # => error: Invalid literal
    ```

- the writer escapes exactly two characters, `"` and `\`. control characters, tabs and newlines
   included, are written inside the quotes verbatim.
    ```nushell
    ["a\nb"] | to nuon
    # => ["a
    # => b"]
    ```
    - legal - the reader accepts a literal newline inside `"..."` - which means a nuon document
       is not line-oriented, and anything that splits it on newlines will get it wrong.

## numbers

- integers: an optional sign then decimal digits. leading zeros are allowed and insignificant.
    ```nushell
    "[007, 000, +5, -0]" | from nuon | to json -r                               # => [7,0,5,0]
    ```
    - `_` is a digit separator, stripped before conversion. accepted in decimal integers, radix
       literals, floats, and the numeric head of a unit literal.
        ```nushell
        "[1_000, 1_0.5, 9_223_372_036_854_775_807]" | from nuon | to json -r     # => [1000,10.5,9223372036854775807]
        ```
    - a decimal integer outside `int` range is **promoted to float**, silently and with loss of
       precision. it is not an error.
        ```nushell
        "9223372036854775808" | from nuon | describe                             # => float
        ```

- radix literals: `0x` hex, `0o` octal, `0b` binary. the prefix letter is **lowercase only** -
   `0X`, `0O` and `0B` are errors - while the digits themselves may be either case.
    ```nushell
    "[0xff, 0o17, 0b101, 0xAB, 0x_f_f]" | from nuon | to json -r                # => [255,15,5,171,255]
    "0XFF" | from nuon                                                          # => error
    ```
    - a sign does **not** combine with a radix literal, and the two signs fail differently:
       `+0xff` is an error, but `-0xff` is the *string* `-0xff`. write a negative hex value and
       you get a string back, with no warning of any kind.
        ```nushell
        "-0xff" | from nuon | describe                                          # => string
        ```
    - always an integer: never promoted to float, unlike a decimal integer. the accepted
       range is the full **unsigned** 64 bits reinterpreted as two's complement, so the upper half
       reads back negative; past 64 bits is an error.
        ```nushell
        "[0x8000000000000000, 0xFFFFFFFFFFFFFFFF]" | from nuon | to json -r
        # => [-9223372036854775808,-1]
        "0x10000000000000000" | from nuon                                       # => error
        ```

- floats: digits with a `.`, an exponent, or both. unlike json, a bare leading or trailing dot is
   accepted.
    ```nushell
    "[1.5, 1., .5, 1e3, 1E-3]" | from nuon | to json -r                         # => [1.5,1.0,0.5,1000.0,0.001]
    ```
    - the writer never emits `1.` or `.5`, so an implementation that only round-trips its own
       output will not discover this. the reader must accept them.
    - non-finite: rust's `f64::from_str` grammar - an optional sign then an **ascii-case-
       insensitive** `nan`, `inf` or `infinity`.
        ```nushell
        "[nan, NAN, nAn, +nan, -nan, inf, Inf, INF, Infinity, -infinity]" | from nuon | describe
        # => list<float>
        ```
        - folding is ascii-only: `İNF` is a string. so are `info`, `infra`, `nano`, `in`, `nan1`,
           `infinityx`.
    - decimal overflow **saturates** to `±inf`, per ieee 754's decimal conversion rule. not an
       error, no fallback to string. underflow is a different case and converts to `0`.
        ```nushell
        "1e400"  | from nuon                                                     # => inf
        "1e-999" | from nuon                                                     # => 0.0
        ```
    - float output **never uses an exponent**. an integral value is its exact decimal expansion
       plus `.0`; anything else is the shortest round-tripping decimal, fully expanded. `-0.0`
       keeps its sign.
        ```nushell
        [1.0 1e-5 1e15 1e16 1e-30 -0.0 0.1] | to nuon
        # => [1.0, 0.00001, 1000000000000000.0, 10000000000000000.0, 0.000000000000000000000000000001, -0.0, 0.1]
        ```
        - `to json` does use an exponent past `1e16`. an implementation sharing one float
           formatter between the two is wrong for one of them.

## durations and filesizes

- durations: a numeric head immediately followed by a unit suffix, decoding to a nanosecond count.
   the suffix table is exact and case-**sensitive**.
    - the common cases: `ns` 1, `us` 1e3, `ms` 1e6, `sec` 1e9, `min` 60e9, `hr` 3600e9, `day`
       86400e9, `wk` 604800e9. full table in [reference tables](#reference-tables).
    - the micro suffix has **two accepted spellings that render identically**: `µs` U+00B5 and
       `μs` U+03BC. an implementation carrying only one will read `1μs` as a string, write it back
       bare, and have nushell read a duration out of what was a string. invisible in a diff.
        ```nushell
        "[1μs, 1µs]" | from nuon | to json -r                                    # => [1000,1000]
        ```
    - not durations: `1s`, `1m`, `1h`, `1d`, `1week`, `1second`.
        ```nushell
        "[1s, 1sec, 1min]" | from nuon | to json -r                              # => ["1s",1000000000,60000000000]
        ```
    - the head may be an integer or float, may be negative, and may contain `_`. a leading `+` is
       **not** accepted: `+2min` is the string `"+2min"`.
        ```nushell
        "[1.5min, -1_000ns]" | from nuon | to json -r                            # => [90000000000,-1000]
        ```
    - overflow is an **error**; it must not wrap. `9999999999999wk` wraps to a negative duration
       in a naive implementation - a positive literal producing a negative value, with no
       diagnostic.
    - the writer emits durations in `ns`, always.
        ```nushell
        [1sec] | to nuon                                                         # => [1000000000ns]
        ```

- filesizes: the same shape, decoding to a byte count, with a **case-insensitive** suffix.
    - `b` is 1; `kb mb gb tb pb eb` are 10^3 .. 10^18; `kib mib gib tib pib eib` are 2^10 .. 2^60.
       full table in [reference tables](#reference-tables).
    - matching is longest-suffix-first, so `1kib` is not `1k` + `ib` and `1kb` is not swallowed by
       the bare `b` entry.
        ```nushell
        "[1KB, 1kB, 1kiB, 1B, 1_000_000b]" | from nuon | to json -r              # => [1000,1000,1024,1,1000000]
        ```
    - `zb`, `yb`, `zib`, `yib` are spellings nushell **knows** and then overflows on, so they are
       hard errors rather than strings.
    - a leading `+` is not accepted. overflow is an error, never a wrap: a negative filesize is
       not a thing and inventing one is worse than refusing.
    - the writer emits filesizes in `b`, always.
        ```nushell
        [1kb] | to nuon                                                          # => [1000b]
        ```

## datetimes

- datetimes: a bare literal, the only one that may contain `:`.
    - the grammar is a strict `YYYY-MM-DD`, optionally followed by `T` `HH:MM:SS`, an optional
       fractional second, and an optional `Z`/`z` or `±HH:MM`.
    - because `:` is otherwise structural, a datetime must be recognised as a **prefix** before
       generic bare-word scanning, which would truncate at `2000-01-01T12`.
    - the fields are **range-checked**, not merely checked for being digits. the day check is
       exact per month and leap-year aware.
        ```nushell
        "[2000-02-29, 2024-02-29]" | from nuon | to json -r
        # => ["2000-02-29T00:00:00+00:00","2024-02-29T00:00:00+00:00"]
        "[2100-02-29, 2000-13-45, 2000-04-31, 2000-1-1]" | from nuon | to json -r
        # => ["2100-02-29","2000-13-45","2000-04-31","2000-1-1"]
        ```
        - a flat 1-31 day limit would turn roughly thirty ordinary strings a year - every 31st
           of a 30-day month, and most of february - into datetimes no other reader accepts.
        - `2000-1-1` is a string: month and day must be two digits.
    - `:60` in the seconds field is accepted as a leap second.
        ```nushell
        "[2000-01-01T23:59:60Z]" | from nuon | to json -r                        # => ["2000-01-01T23:59:60+00:00"]
        ```
    - the offset is range-checked too; `+99:99` is not an offset and the whole token stays a
       string.
        ```nushell
        "[2000-01-01T12:34:56+99:99]" | from nuon | to json -r                   # => ["2000-01-01T12:34:56+99:99"]
        ```
    - a date-only literal defaults to `00:00:00` and `+00:00`. the writer always emits the full
       rfc-3339 form.
        ```nushell
        {date: 2000-01-01} | to nuon                                             # => {date: 2000-01-01T00:00:00+00:00}
        ```

## binary, cell-paths and ranges

- these three have literal syntax and round-trip through the writer, so a reader that skips them
   cannot read everything `to nuon` produces. `--serialize` does not affect them; they are
   written natively whether or not it is given.

- binary: `0x[` hex digits `]`. whitespace and commas between digits are ignored, an odd digit
   count is left-padded, and `0b[` bits `]` is also accepted. there is no octal form.
    ```nushell
    "[0x[be ef], 0x[beef], 0x[BE, EF], 0x[b], 0b[1010], 0x[]]" | from nuon | to nuon
    # => [0x[BEEF], 0x[BEEF], 0x[BEEF], 0x[0B], 0x[0A], 0x[]]
    "[0o[777]]" | from nuon                                                     # => error
    ```
    - the writer always emits `0x[`, uppercase hex, no separators.

- cell-path: `$` followed by `.` and a member, repeated. a member is a bare word, a quoted
   string, or an integer index, and a trailing `?` marks it optional.
    ```nushell
    "[$.a.b, $.0, $.a.0.b, $.'a b', $.a?, $.]" | from nuon | to nuon
    # => [$.a.b, $.0, $.a.0.b, $."a b", $.a?, $.]
    ```
    - the writer double-quotes a member that would not survive as a bare word.

- range: `start..end` inclusive, or `start..<end` exclusive. either bound may be omitted. a
   middle value gives the **second element**, not the step, so `1..3..9` counts by two.
    ```nushell
    "[1..5, 1..<5, 1.., ..5, 1..3..9, -5..5, 1.5..2.5]" | from nuon | to nuon
    # => [1..5, 1..<5, 1.., 0..5, 1..3..9, -5..5, 1.5..2.5]
    ```
    - an omitted start is written back as `0`, so `..5` and `0..5` are the same value.

## lists and records

- lists: `[` value* `]`. items are separated by `,` **or** by whitespace; a trailing comma is
   allowed.
    ```nushell
    "[[1,2,3], [1 2 3], [1, 2, 3,]]" | from nuon | to json -r                    # => [[1,2,3],[1,2,3],[1,2,3]]
    ```

- records: `{` (key `:` value)* `}`. pairs are separated by `,` or whitespace; a trailing comma is
   allowed.
    ```nushell
    "{a: 1 b: 2}" | from nuon | to json -r                                       # => {"a":1,"b":2}
    ```
    - a key is a bare word, any quoted string form, or a raw string.
    - keys are quoted by the same rule as values, with one exception: `--raw-strings` applies to
       values only.
        ```nushell
        {"a b": 1, "c.d": 2, "e": 3} | to nuon                                   # => {"a b": 1, "c.d": 2, e: 3}
        {"k\"": 1} | to nuon --raw-strings                                       # => {"k\"": 1}
        ```
    - keys are ordered and the order is preserved on read and on write. it is part of the value.
    - a duplicate key is an **error**, not a last-write-wins merge.
        ```nushell
        "{a: 1, a: 2}" | from nuon                                               # => error: column_defined_twice
        ```
        - a real difference from json, where duplicate keys are conventionally resolved rather
           than refused. an implementation sharing its object builder with a json reader must not
           share this rule.

## table form

- table form: the reason nuon is not just json with fewer commas.
    - syntax: `[` header `;` row (`,`|ws) row ... `]`, where the header and every row are lists.
        ```nushell
        "[[a b]; [1 2] [3 4]]" | from nuon | to json -r                          # => [{"a":1,"b":2},{"a":3,"b":4}]
        ```
    - it denotes exactly a list of records: each row pairs the header names with the row's cells
       positionally. it is not a distinct type, and once decoded a table is indistinguishable from
       a list of records.
    - the header must be a list of **strings**; every row must be a list of the **same length**
       as the header.
        ```nushell
        "[[a 1]; [1 2]]" | from nuon                                             # => error
        "[[a b]; [1]]"   | from nuon                                             # => error
        ```
    - a **duplicate column name is refused**.
        ```nushell
        "[[a b a]; [1 2 3]]" | from nuon                                         # => error: column_defined_twice
        ```
        - this is where sharing a last-write-wins record builder is catastrophic rather than
           merely wrong: `[[a, b, a]; [1, 2, 3]]` would decode as `[{a: 3, b: 2}]`, deleting one
           cell and reporting another under a column it was never written in.
    - **a table is not a row.** the `;` form may not appear where a row is expected.
        ```nushell
        "[[a b]; [[c]; [1], [2]]]" | from nuon                                   # => error
        ```
        - a properly bracketed row whose single **cell** is a table is legal:
            ```nushell
            "[[a]; [[[b]; [1]]]]" | from nuon | to json -r                       # => [{"a":[{"b":1}]}]
            ```
        - only the parser can make this distinction, since after decoding a table and a list
           literal are both lists.
    - **a header with no rows is an error.** an empty table is not expressible in the table form,
       so `[]` is the only way to write one. reading `[[a b];]` as a one-element list containing
       the header is a silently wrong value rather than a refusal.
    - **when the writer chooses the table form**: only if the list is non-empty, every element is
       a record, the records have at least one key, and every record has the **same keys in the
       same order**. differing order alone disqualifies it.
        ```nushell
        [{a: 1, b: 2}, {b: 3, a: 4}] | to nuon                                   # => [{a: 1, b: 2}, {b: 3, a: 4}]
        [{}, {}]                     | to nuon                                   # => [{}, {}]
        ```
        - `to nuon --list-of-records` suppresses the table form unconditionally.

## output

- the writer's flags and what each one defaults to. every one is off unless given, so the default
   output is compact and on a single line.

    | flag | short | default | effect |
    | --- | --- | --- | --- |
    | `--raw` | `-r` | off | remove all whitespace |
    | `--indent N` | `-i` | off | one item per line, `N` spaces per level |
    | `--tabs N` | `-t` | off | one item per line, `N` tabs per level |
    | `--pretty` | `-p` | off | identical to `--indent 2` |
    | `--list-of-records` | `-l` | off | never use the table form |
    | `--no-commas` | `-c` | off | separate with whitespace instead of `,`; excludes `--raw` |
    | `--raw-strings` | `-R` | off | emit string values as `r#'...'#` |
    | `--serialize` | `-s` | off | emit types that cannot be read back |

    ```nushell
    [[a b]; [1 2] [3 4]] | to nuon                                              # => [[a, b]; [1, 2], [3, 4]]
    ```

- the four output styles of `to nuon`:
    - default (compact): no newlines, `, ` between items, `: ` after a key.
    - `--raw` / `--indent 0`: as compact, spaces removed.
    - `--indent N` / `--tabs N`: one item per line, `N` units of indentation per level.
    - `--pretty`: identical to `--indent 2`.

- precedence when flags conflict. the order they are written in never matters.
    - `--raw` wins over `--indent`, `--tabs` and `--pretty`.
        ```nushell
        [[a b]; [1 2]] | to nuon --pretty --raw                                 # => [[a,b];[1,2]]
        [[a b]; [1 2]] | to nuon --raw --indent 4                               # => [[a,b];[1,2]]
        ```
    - `--tabs` wins over `--indent`.
        ```nushell
        [[a b]; [1 2]] | to nuon --indent 4 --tabs 1 | lines | get 1 | str starts-with (char tab)
        # => true
        ```
    - `--indent 0` and `--tabs 0` mean `--raw`.
        ```nushell
        [[a b]; [1 2]] | to nuon --indent 0                                     # => [[a,b];[1,2]]
        ```
    - `--raw` does **not** override `--list-of-records` or `--raw-strings`. those compose with it.
        ```nushell
        [[a b]; [1 2]] | to nuon --raw --list-of-records                        # => [{a:1,b:2}]
        ```

- each switch also takes an explicit value, so a default can be set or cleared by name.
    ```nushell
    [[a b]; [1 2]] | to nuon --pretty=false                                     # => [[a, b]; [1, 2]]
    ```

- `--indent` aligns table columns even though only `--pretty` advertises it - they are the same
   code path.
    ```nushell
    [[a b]; [1 2] [3 4]] | to nuon --indent 4
    # => [
    # =>     [a, b];
    # =>     [1, 2],
    # =>     [3, 4]
    # => ]
    ```

- `--list-of-records` prints each record **inline on one line** even under indentation.
    ```nushell
    [[a, b]; [1, 2], [3, 4]] | to nuon --list-of-records --indent 2
    # => [
    # =>   {a: 1, b: 2},
    # =>   {a: 3, b: 4}
    # => ]
    ```

- `--raw-strings` emits a string value as a raw string **only when it contains a `"` or a `\`**.
   every other string is written as it would be without the flag. values only, not keys.
    ```nushell
    'hello "world"' | to nuon --raw-strings                                      # => r#'hello "world"'#
    'hello world'   | to nuon --raw-strings                                      # => "hello world"
    "has 'single'"  | to nuon --raw-strings                                      # => "has 'single'"
    ```
    - the name suggests "write every string raw", which is what an implementation copying the
       flag will do. single quotes, backticks and newlines do not trigger the raw form.

- a lone string at the top level is always quoted, because a bare word there would be ambiguous.
    ```nushell
    "hi"   | to nuon                                                             # => "hi"
    ["hi"] | to nuon                                                             # => [hi]
    ```

- `--raw` and `--no-commas` are mutually exclusive. `--no-commas` uses whitespace as the
   separator and `--raw` removes whitespace, so the two ask for opposite things. accepting both
   leaves no separator at all, which silently changes the value: `[1 2 3]` reads back as the
   integer `123`.

- an empty container under indentation emits no blank line: `{a: {}}` renders `a: {}`.

- table column alignment under indentation uses **one** measure for both the column width and the
   padding.
    - the column width is the maximum, over the header cell and every body cell of that column, of
       the cell's rendered width.
    - the padding written after a cell is `width - measure(cell)` spaces, then one more space.
       padding goes **after** the separator, so commas stay flush against their cells:
       `[name,  age]`, not `[name , age]`.
    - measuring the width one way and the padding another makes cjk columns visibly wrong. the
       measure has to be the same on both sides; terminal display width is the one that makes a
       table look aligned, since `日` is one rune but two columns wide.
    - a cell that renders multi-line participates in the width calculation with its full rendered
       length, newlines included. nested containers under `--pretty` render at `depth + 2`.
        ```nushell
        [[a]; [{x: 1}]] | to nuon --pretty
        # => [
        # =>   [a];
        # =>   [{
        # =>       x: 1
        # =>     }]
        # => ]
        ```

## resource limits

- resource limits - for implementers, not part of the format:
    - nuon nests arbitrarily; a naive recursive-descent reader blows the native stack on
       `[[[[...`. impose a nesting cap.
    - the **writer must not emit what the reader will refuse**, and the cap must mean the same
       thing for lists and records. check depth at the opening bracket, not at the next value -
       an empty innermost list never descends again, so a check placed one level later silently
       grants lists one extra level records do not get.

- prior art, such as it is: the book gives eight sentences and one example; `crates/nuon/README.md`
   gives four bullets and a before/after; `from nuon`'s help has three examples and no prose.
   `lang-guide` recommends the media type `application/x-nuon` and notes there is no iana
   registration. there is no grammar file, no rfc, and no format version separate from nushell's.
   every third-party artefact found is a republish or fork of the same rust crate.

## reference tables

- duration suffixes - case-**sensitive**, exact:
    | suffix | nanoseconds |
    | --- | --- |
    | `ns` | 1 |
    | `us` | 1 000 |
    | `µs` | 1 000 - U+00B5 micro sign |
    | `μs` | 1 000 - U+03BC greek small letter mu |
    | `ms` | 1 000 000 |
    | `sec` | 1 000 000 000 |
    | `min` | 60 000 000 000 |
    | `hr` | 3 600 000 000 000 |
    | `day` | 86 400 000 000 000 |
    | `wk` | 604 800 000 000 000 |
    - `µs` and `μs` are **two different codepoints that render identically** and both are
       accepted. an implementation carrying only one will read `1μs` as a string, write it back
       bare, and have nushell read a duration out of what was a string. invisible in a diff.
        ```nushell
        "[1μs, 1µs]" | from nuon | to json -r                                    # => [1000,1000]
        ```
    - not durations: `1s`, `1m`, `1h`, `1d`, `1week`, `1second`.
        ```nushell
        "[1s, 1sec, 1min]" | from nuon | to json -r                              # => ["1s",1000000000,60000000000]
        ```

- filesize suffixes - case-**insensitive**:
    | suffix | bytes |
    | --- | --- |
    | `b` | 1 |
    | `kb` | 10^3 |
    | `mb` | 10^6 |
    | `gb` | 10^9 |
    | `tb` | 10^12 |
    | `pb` | 10^15 |
    | `eb` | 10^18 |
    | `kib` | 2^10 |
    | `mib` | 2^20 |
    | `gib` | 2^30 |
    | `tib` | 2^40 |
    | `pib` | 2^50 |
    | `eib` | 2^60 |
    - matching is longest-suffix-first, so `1kib` is not `1k` + `ib` and `1kb` is not swallowed
       by the bare `b` entry.
        ```nushell
        "[1KB, 1kB, 1kiB, 1B, 1_000_000b]" | from nuon | to json -r              # => [1000,1000,1024,1,1000000]
        ```
    - `zb`, `yb`, `zib`, `yib` are spellings nushell **knows** and then overflows on, so they are
       hard errors rather than strings.

- the quote-forcing set - exactly these bytes, and no others:
    ```
    space  tab  cr  lf  !  "  #  $  '  (  )  ,  .  :  ;  =  ?  `  [  ]  {  |  }
    ```
    - `.` and `$` are there because nushell gives them meaning (cell paths, variables), not
       because they are punctuation.
    - **not** in the set: `-`, `/`, `\`, `%`, `&`, `*`, `+`, `<`, `>`, `@`, `^`, `_`, `~`, and
       every non-ascii byte.

## conformance

- checklist, in the order things actually go wrong:
    - bare word `#` handling - `[a#b]` is `["a#b"]`, `[a, # x\n b]` is `["a","b"]`.
    - the digit rule on the writer - category `Nd`, any position, runes not bytes.
    - `0x`/`0o`/`0b` written bare produce unreadable files, not misread values.
    - both micro-sign codepoints, `µ` U+00B5 and `μ` U+03BC, in the duration table.
    - datetime fields range-checked exactly, leap years included.
    - integer overflow promotes to float; radix and unit overflow are errors.
    - float overflow saturates to `±inf`; underflow goes to zero.
    - float output never uses an exponent.
    - duplicate record keys and duplicate table columns are refused.
    - a table is not a table row.
    - record key order is part of the value.
    - `binary`, `cell-path` and `range` have literal syntax and round-trip; a reader that skips
       them cannot read everything the writer emits.
    - table width and padding use the same measure, so cjk columns line up.
    - a leading bom is stripped, exactly one, and only at the start.

- open questions - things this document could not establish:
    - is there any stability commitment for nuon across nushell releases? nothing in the book,
       the crate readme or the changelog states one. the unit tables, the quote-forcing set and
       the float format have all moved between releases and nothing marks them as stable.
    - the exact upper bound of the duration and filesize suffix tables. `zb`, `yb`, `zib` and
       `yib` are recognised well enough to produce an overflow error rather than a string, but
       whether there are further spellings, and whether any can decode successfully at small
       magnitudes, was not established. the safe implementation quotes any number followed by two
       or more ascii letters and does not try to track the table.
    - whether the digit rule is exactly general category `Nd` or a nearby predicate. roughly 45
       probes were consistent with `Nd` - `¹`, `²`, `③` and `½` (category `No`) stay bare - but
       the space was not exhausted.
    - whether a **multi-line** cell's column width is its full byte length including newlines, or
       its widest line. the only multi-line-cell example available was single-column, where no
       padding is emitted, so the two hypotheses are indistinguishable.
    - whether the fractional-second field of a datetime has a precision limit, and what happens
       past it.
    - whether a `\r\n` inside a `"..."` string is preserved verbatim or normalised.
    - what, if anything, `to nuon --serialize` guarantees. it is documented as one sentence and
       its output is explicitly not required to read back. it renders `closure` and `block` as
       strings, which do not read back as the original type. it does **not** affect `binary`,
       `cell-path` or `range`, which have literal syntax and round-trip with or without it.
