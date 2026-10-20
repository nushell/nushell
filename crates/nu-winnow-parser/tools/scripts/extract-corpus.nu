#!/usr/bin/env nu
# Collect Nushell snippets from the places Nushell itself documents its
# syntax, as JSON lists of `{origin, snippet}` records:
#
#   nu-command-examples.json   every `Example { example: ".." }` in the
#                              command crates of a Nushell checkout
#   book.json                  every ```nu / ```nushell code block in the
#                              English chapters of nushell.github.io
#
# `tests/examples.rs` parses the first list and requires zero diagnostics;
# `tools/nushell-harness`'s `differential` runs both lists through nu-parser
# and this parser and reports where they disagree.
#
#   nu tools/scripts/extract-corpus.nu                       # both, into tests/corpus/snippets
#   nu tools/scripts/extract-corpus.nu --book ~/src/nushell.github.io

# Decode the escapes of a plain (non-raw) Rust string literal.
def unescape-rust [] : string -> string {
    # Escaped backslashes are set aside first so that `\\n` stays a backslash
    # followed by `n` instead of becoming a newline.
    $in
    | str replace -a -r '\\\n\s*' ''    # a `\` at the end of a line continues it
    | str replace -a '\\' "\u{0}"
    | str replace -a -r '\\n' "\n"
    | str replace -a -r '\\t' "\t"
    | str replace -a -r '\\(.)' '$1'
    | str replace -a "\u{0}" '\'
}

# The `example:` string literals of one Rust source file.
def examples-in [file: path, root: path]: nothing -> list<record<origin: string, snippet: string>> {
    let origin = $file | path relative-to $root
    open --raw $file
    | parse -r '(?s)example:\s*(?:r##"(?P<raw2>.*?)"##|r#"(?P<raw1>.*?)"#|r"(?P<raw0>[^"]*)"|"(?P<plain>(?:[^"\\]|\\.)*)")'
    | each {|m|
        let snippet = if ($m.raw2 | is-not-empty) {
            $m.raw2
        } else if ($m.raw1 | is-not-empty) {
            $m.raw1
        } else if ($m.raw0 | is-not-empty) {
            $m.raw0
        } else {
            $m.plain | unescape-rust
        }
        {origin: $origin, snippet: $snippet}
    }
    | where ($it.snippet | str trim | is-not-empty)
}

# Every `Example { example: .. }` in the command crates of a Nushell checkout.
export def nu-command-examples [nushell: path]: nothing -> list<record<origin: string, snippet: string>> {
    let crates = [nu-cmd-lang nu-command nu-cmd-extra nu-cmd-plugin nu-cmd-base]
    $crates
    | each {|c| glob ($nushell | path join crates $c 'src/**/*.rs') }
    | flatten
    | sort
    | each {|f| examples-in $f $nushell }
    | flatten
}

# Every ```nu / ```nushell fenced block in the English chapters of the book.
export def book-snippets [book: path]: nothing -> list<record<origin: string, snippet: string>> {
    let dirs = [book cookbook lang-guide contributor-book]
    $dirs
    | each {|d| glob ($book | path join $d '**/*.md') }
    | flatten
    | sort
    | each {|f|
        let origin = $f | path relative-to $book
        open --raw $f
        | parse -r '(?s)```nu(?:shell)?[^\n]*\n(?P<code>.*?)```'
        | each {|m| {origin: $origin, snippet: $m.code} }
    }
    | flatten
    | where ($it.snippet | str trim | is-not-empty)
}

export def main [
    --nushell: path      # a Nushell checkout (default: the enclosing workspace)
    --book: path         # a nushell.github.io checkout (default ~/src/nushell.github.io)
    --out: path          # output directory (default tests/corpus/snippets)
]: nothing -> record<examples: int, book: int> {
    let root = if ("tests/fixtures" | path exists) { $env.PWD } else { $env.FILE_PWD | path join ../.. | path expand }
    let nushell = $nushell | default ($root | path join ../.. | path expand)
    let book = $book | default ("~/src/nushell.github.io" | path expand)
    let out = $out | default ($root | path join tests/corpus/snippets)
    mkdir $out
    let examples = nu-command-examples $nushell
    $examples | to json --indent 1 | save -f ($out | path join nu-command-examples.json)
    let blocks = if ($book | path exists) { book-snippets $book } else { [] }
    if ($blocks | is-not-empty) {
        $blocks | to json --indent 1 | save -f ($out | path join book.json)
    }
    {examples: ($examples | length), book: ($blocks | length)}
}
