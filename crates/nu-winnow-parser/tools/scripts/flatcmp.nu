#!/usr/bin/env nu
# Compare Nushell's own `ast --flatten` classification with nu-winnow-parser's
# `flatten()` output, for every file given.
#
# Both sides are mapped onto a coarse class alphabet and compared segment by
# segment over the source bytes:
#   call     command and keyword heads
#   string   strings, bare words, definition names, cell-path names
#   var      variables and declarations
#   literal  ints, floats, bools, null, datetimes, binaries, units
#   op       operators, pipes, ranges, redirections, booleans
#   flag     --flags
#   delim    brackets, braces and punctuation of collections, blocks, closures
#   sig      def/extern signatures
#   any      wildcard: unknown-command arguments, match patterns, garbage
# Whitespace and bytes inside nu's garbage regions are ignored.
#
# Requires the example binary: `cargo build --release --example parse`.
#
#   nu tools/scripts/flatcmp.nu --commands tools/scripts/std_commands.txt ...(glob ../nu-std/**/*.nu)

const NU_MAP = {
    shape_internalcall: call, shape_keyword: call, shape_external: call, shape_external_resolved: call,
    shape_string: string, shape_raw_string: string, shape_string_interpolation: string,
    shape_globpattern: string, shape_filepath: string, shape_directory: string, shape_glob_interpolation: string,
    shape_variable: var, shape_vardecl: var,
    shape_int: literal, shape_float: literal, shape_bool: literal, shape_nothing: literal,
    shape_datetime: literal, shape_binary: literal, shape_literal: literal,
    shape_operator: op, shape_pipe: op, shape_redirection: op, shape_and: op, shape_or: op, shape_range: op,
    shape_flag: flag,
    shape_list: delim, shape_record: delim, shape_table: delim, shape_block: delim, shape_closure: delim,
    shape_signature: sig,
    shape_externalarg: any, shape_matchpattern: any, shape_custom: any,
}

const OURS_MAP = {
    InternalCall: call, Keyword: call, External: call, Attribute: call,
    Definition: string, String: string, StringInterpolation: string,
    Variable: var, VarDecl: var,
    Int: literal, Float: literal, Bool: literal, Nothing: literal, DateTime: literal, Binary: literal,
    Filesize: literal, Duration: literal,
    Operator: op, Pipe: op, Redirection: op, Range: op, Boolean: op,
    Flag: flag,
    List: delim, Record: delim, Table: delim, Block: delim, Closure: delim, Signature: delim,
    ExternalArg: any, MatchPattern: any, Type: any,
}

const UNITS = [b kb mb gb tb pb eb kib mib gib tib pib eib ns us µs μs ms sec min hr day wk]

# Nu's rows as sorted `{start, end, class}` intervals plus the garbage regions.
def nu-intervals [src: string]: nothing -> record<intervals: list<any>, garbage: list<any>> {
    let rows = ast --flatten $src | sort-by span.start span.end
    let dups = $rows | each { $"($in.span.start)-($in.span.end)" } | uniq --count | where count > 1 | get value
    let n = $rows | length
    mut out = []
    mut garbage = []
    mut i = 0
    while $i < $n {
        let r = $rows | get $i
        let s = $r.span.start
        let e = $r.span.end
        if $r.shape == "shape_garbage" {
            $garbage = ($garbage | append {start: $s, end: $e})
            $i += 1
            continue
        }
        mut c = $NU_MAP | get -o $r.shape | default any
        if $"($s)-($e)" in $dups { $c = "any" }
        # `1kb` is an int followed by a string unit at the same position.
        if $c == "literal" and ($i + 1) < $n {
            let nxt = $rows | get ($i + 1)
            if $nxt.span.start == $e and $nxt.shape == "shape_string" and (($nxt.content | str lowercase) in $UNITS) {
                $out = ($out | append {start: $s, end: $nxt.span.end, class: "literal"})
                $i += 2
                continue
            }
        }
        $out = ($out | append {start: $s, end: $e, class: $c})
        $i += 1
    }
    {intervals: $out, garbage: $garbage}
}

# nu-winnow-parser's rows as sorted `{start, end, class}` intervals.
def ours-intervals [parse: path, file: path, bytes: binary]: nothing -> list<any> {
    let rows = ^$parse --flat $file | from tsv --noheaders | rename start end shape
    let sigs = $rows | where shape in [sig closure-sig]
    let plain = $rows
        | where shape not-in [sig closure-sig]
        | each {|r|
            if $r.shape == "Comment" or $r.shape == "Garbage" {
                null
            } else if $r.shape == "CellPath" {
                let text = $bytes | bytes at $r.start..<$r.end | decode utf8 | str replace -r '[?!]+$' ''
                let class = if ($text =~ '^\d+$') { "literal" } else { "string" }
                {start: $r.start, end: ($r.start + ($text | encode utf8 | bytes length)), class: $class}
            } else {
                {start: $r.start, end: $r.end, class: ($OURS_MAP | get -o $r.shape | default any)}
            }
        }
        | compact
    # A signature is compared as one unit; closure parameter lists count as delimiters.
    let outside = $plain | where {|r| not ($sigs | any {|s| $s.start <= $r.start and $r.end <= $s.end }) }
    let sig_rows = $sigs | each {|s| {start: $s.start, end: $s.end, class: (if $s.shape == "sig" { "sig" } else { "delim" })} }
    $outside | append $sig_rows | sort-by start end
}

# Walk both interval lists over the source and report runs whose classes differ.
def differences [src: string, theirs: list<any>, garbage: list<any>, ours: list<any>]: nothing -> list<any> {
    let bytes = $src | encode utf8
    let n = $bytes | bytes length
    let na = $theirs | length
    let nb = $ours | length
    mut ai = 0
    mut bi = 0
    mut pos = 0
    mut diffs = []
    while $pos < $n {
        while $ai < $na and ($theirs | get $ai).end <= $pos { $ai += 1 }
        while $bi < $nb and ($ours | get $bi).end <= $pos { $bi += 1 }
        let a = if $ai < $na { $theirs | get $ai } else { null }
        let b = if $bi < $nb { $ours | get $bi } else { null }
        let a_cur = if $a != null and $a.start <= $pos { $a } else { null }
        let b_cur = if $b != null and $b.start <= $pos { $b } else { null }
        let a_next = if $a_cur != null { $a_cur.end } else if $a != null { $a.start } else { $n }
        let b_next = if $b_cur != null { $b_cur.end } else if $b != null { $b.start } else { $n }
        let next = [$a_next $b_next $n] | math min
        let next = if $next <= $pos { $pos + 1 } else { $next }
        let ca = if $a_cur != null { $a_cur.class } else { null }
        let cb = if $b_cur != null { $b_cur.class } else { null }
        if $ca != null and $cb != null and $ca != $cb and $ca != "any" and $cb != "any" {
            let text = $bytes | bytes at $pos..<$next | decode utf8
            let in_garbage = $garbage | any {|g| $g.start < $next and $pos < $g.end }
            # Known classification conventions that are not parse differences: nu labels
            # the `=` of `alias` as a call and the `^` of an external call as a delimiter.
            let convention = ($text | str trim) in ["=" "^"]
            if not ($text | str trim | is-empty) and not $in_garbage and not $convention {
                let line = ($bytes | bytes at 0..<$pos | decode utf8 | lines | length)
                $diffs = ($diffs | append {line: $line, text: $text, nu: $ca, ours: $cb})
            }
        }
        $pos = $next
    }
    $diffs
}

# Compare nu's `ast --flatten` classification with nu-winnow-parser's for each file.
def main [
    --parse: path                # the `parse` example binary (default: <workspace target>/release/examples/parse)
    --commands: path             # extra command names for the parser (see std_commands.txt)
    --max-per-file: int = 12     # how many differences to print per file
    ...files: path               # `.nu` files to compare
] {
    let parse = $parse | default (($env.CARGO_TARGET_DIR? | default ($env.FILE_PWD | path join ../../../../target) | path expand) | path join release/examples/parse)
    let env_extra = if $commands == null { {} } else { {NU_WINNOW_COMMANDS: ($commands | path expand)} }
    mut with_diffs = 0
    mut total_diffs = 0
    for file in $files {
        let src = open --raw $file
        let theirs = nu-intervals $src
        let ours = with-env $env_extra { ours-intervals $parse $file ($src | encode utf8) }
        let diffs = differences $src $theirs.intervals $theirs.garbage $ours
        if ($diffs | is-not-empty) {
            $with_diffs += 1
            $total_diffs += ($diffs | length)
            print $"== ($file): ($diffs | length) differences"
            for d in ($diffs | first $max_per_file) {
                print $"  line ($d.line): ($d.text | to json): nu=($d.nu) ours=($d.ours)"
            }
        }
    }
    print $"($files | length) files, ($with_diffs) with differences, ($total_diffs) differing runs"
}
