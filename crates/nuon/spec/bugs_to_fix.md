# NUON bugs to fix, as of nushell 0.115.1

what nushell 0.115.1 does today where it differs from
[nuon_formal_specification](./nuon_formal_specification.md). tick one off when it is fixed. when
the list is empty, this file goes away and so does the todo at the top of the spec.

1. [ ] bug 1 - a leading utf-8 bom is rejected. strip one instead.
1. [ ] bug 2 - the empty document decodes to `null`. error instead.
1. [ ] bug 3 - `[[a b];]` is a list containing the header, not an empty table. error instead.
1. [ ] bug 4 - an empty container under indentation emits a blank line. do not emit it.
1. [ ] bug 5 - `--raw --no-commas` emits no separator and loses data. make the two exclusive.
1. [ ] bug 6 - table width is measured in bytes but padded in runes. use one measure for both.
1. [ ] bug 7 - `inf` and `NaN` become `null` through `to json`. not a nuon bug, and not fixable
   in this crate, since json cannot spell them.
1. [ ] bug 8 - duration overflow saturates or wraps instead of erroring. `1e30sec` is a
   **negative** duration.
1. [ ] bug 9 - filesizes may be negative, and an oversized one saturates instead of erroring.

each entry below shows what 0.115.1 actually does. an implementation that has to match nushell
byte for byte still has to reproduce these until they are fixed.

- bug 1 - a leading utf-8 byte order mark (`EF BB BF`) is **rejected**.
    ```nushell
    "\u{feff}1"   | from nuon                                                    # => error: calls not supported in nuon
    "\u{feff}[1]" | from nuon                                                    # => error: calls not supported in nuon
    ```
    - `U+FEFF` is neither whitespace nor a delimiter, so it glues onto the first token and the
       result parses as a command call.
    - files acquire a bom from windows editors and from powershell's `Out-File`, so this is
       normal input, not something malicious.
    - **do not reproduce.** strip exactly one leading bom, and only a leading one - `U+FEFF`
       elsewhere is legitimate content. this accepts a bit more than nushell does, and looks
       like a gap that should be closed upstream.

- bug 2 - the empty document decodes to `null`.
    ```nushell
    "" | from nuon | describe                                                    # => nothing
    ```
    - an empty byte sequence is not a value in json and arguably not one here. rejecting it is
       defensible: nothing in nushell relies on the current behaviour, and a caller cannot
       distinguish "empty file" from "file containing null".

- bug 3 - a table header with no rows degenerates to the header itself.
    ```nushell
    "[[a b];]" | from nuon | to json -r                                          # => [["a","b"]]
    ```
    - not an empty table - a list containing the header list. an empty table is not expressible
       in the table form; `[]` must be used.
    - **reproduce**, for compatibility. documenting it as an intended feature would be dishonest.

- bug 4 - an empty container under indentation emits a blank line.
    ```nushell
    {a: {}, b: []} | to nuon --indent 2
    # => {
    # =>   a: {
    # =>
    # =>   },
    # =>   b: [
    # =>
    # =>   ]
    # => }
    ```
    - the opening newline is emitted before the item loop and the closing newline after it,
       unconditionally, so a zero-item container gets both.
    - valid nuon, reads back correctly. reproduce only if byte-identity with nushell is required.

- bug 5 - `--raw --no-commas` emits no separator at all. **this one loses data.**
    ```nushell
    [1 2 3]           | to nuon --raw --no-commas                                # => [123]
    [true false null] | to nuon --raw --no-commas                                # => [truefalsenull]
    [[name age]; [Alice 30] [Bob 25]] | to nuon --raw --no-commas
    # => [[nameage];[Alice30][Bob25]]
    ```
    - `--raw` removes the space after the comma, `--no-commas` removes the comma, and the two
       compose by subtraction into nothing.
    - not a formatting wart: `[1 2 3]` reads back as the single integer `123`; `[true false null]`
       reads back as the single **string** `"truefalsenull"`, changing both arity and types; the
       table loses its column boundaries and reads back as a one-column table named `nameage`.
       every result is valid nuon and parses cleanly, so nothing downstream can detect the loss.
    - **do not reproduce.** the two flags contradict each other: `--no-commas` uses whitespace as
       the separator and `--raw` removes whitespace. rejecting the combination costs nothing,
       because the alternative fix, emitting one space, produces output the same size as `--raw`
       alone: dropping a comma and adding a space is a one-for-one byte swap. so the combination
       has no size advantage over `--raw` once it is correct, and the only thing it buys today is
       the corrupt output.

- bug 6 - table column width is measured in bytes but padding is counted in runes, and the
   disagreement is observable.
    ```nushell
    [[a, b]; ["日本", 1], ["abcde", 2]] | to nuon --pretty
    # => [
    # =>   [a,      b];
    # =>   [日本,     1],
    # =>   [abcde,  2]
    # => ]
    ```
    - the cells are 1, 2 and 5 runes. a rune-max would be 5 and the second field would land at
       rune index 10; nushell puts it at 11, which requires a width of 6 - and 6 is `日本`'s
       **byte** length. the padding it then emits for that cell subtracts runes.
    - bytes for both is wrong the other way: `ab…` is 3 runes and 5 bytes, so byte-padding writes
       that row two spaces short and shifts every column to its right two places left, on that
       row only. the result looks like corruption, not like a column being slightly out.
    - runes for both agrees with nushell on ascii, on `…` and on emoji, and diverges exactly on
       cjk - where bytes and runes stop being a constant factor apart.
    - terminal display width is a third measure and matches nushell nowhere: `🎉` is one rune and
       two terminal columns, and nushell pads it as one.
        ```nushell
        [[a, b]; ["🎉", 1], ["abcde", 2]] | to nuon --pretty
        # => [
        # =>   [a,     b];
        # =>   [🎉,     1],
        # =>   [abcde, 2]
        # => ]
        ```
    - consequence: an implementation cannot both make cjk tables look aligned in a terminal and
       stay byte-identical to nushell. **reproduce** if byte-identity matters, and do not pretend
       the result is correct alignment. fixing this upstream - picking one measure, most plausibly
       terminal width - would be a formatting change only and would break no document.

- bug 7 - `inf`, `-inf` and `NaN` survive a nuon round trip but come out as `null` from
   `to json`. that is a json limitation rather than a nuon one. it matters because it means you
   cannot use json to check how nuon handles floats.

- bug 8 - duration overflow does not error. it does one of three things depending on the unit.
    ```nushell
    "[1e30ns]"  | from nuon | to json -r                                         # => [9223372036854775807]
    "[1e30sec]" | from nuon | to json -r                                         # => [-1000000]
    "[1e30wk]"  | from nuon                                                      # => error
    ```
    - `ns` and `us` saturate to `int` max. `ms`, `sec`, `min`, `hr` and `day` **wrap**, so a
       positive literal reads back as a negative duration. only `wk` errors.
    - the wrapping cases are the serious ones: nothing about `1e30sec` suggests the answer should
       be negative, and the result is a valid duration that no downstream check will question.
    - **do not reproduce.** error on overflow. the spec already says overflow must not wrap; this
       is the implementation not matching it yet.

- bug 9 - filesizes may be negative, and oversized ones saturate rather than erroring.
    ```nushell
    "[-1kb]" | from nuon | to json -r                                            # => [-1000]
    "[1e30b]" | from nuon | to json -r                                           # => [9223372036854775807]
    "[1e30eb]" | from nuon                                                       # => error
    ```
    - `Filesize` is a signed `i64`, so `-1kb` is accepted and decodes to `-1000`. a negative
       count of bytes is not a meaningful value.
    - an out-of-range literal with a small suffix is cast through `f64` and saturates instead of
       failing, while a large suffix errors, so the same magnitude behaves differently depending
       on how it is spelled.
    - **do not reproduce.** refuse negatives, and error on overflow rather than saturating.
