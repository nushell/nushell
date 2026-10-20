#!/usr/bin/env nu
# Generate `grammar/grammar.bnf` and `grammar/grammar.ebnf` from the ```ebnf
# fences of `grammar/grammar.md`, in order.
#
#   nu tools/scripts/gen-grammar.nu                    # rewrite both files
#   nu tools/scripts/gen-grammar.nu --check            # exit 1 when they are stale
#   ebnf2railroad --lint grammar/grammar.ebnf --no-target
#   ebnf2railroad --title "Nushell Grammar" grammar/grammar.ebnf -o grammar/grammar.html
#
# grammar.bnf is the fences verbatim (the notation section as comments).
# grammar.ebnf is the same grammar in ISO 14977 style, which ebnf2railroad and
# the VS Code EBNF extension understand:
#   <a-b> ::= x y | z        a_b = x, y | z ;
#   1*x  N*x  1*6x           x, { x }   N * x   x, 5 * [ x ]
#   "\""  "\\"               '"'  "\"
#   <words with spaces>      ? words with spaces ?   (a rule stated in prose)
#   ; comment                (* comment *)
# A rule whose right-hand side is prose (`<call> over ALL remaining items ...`)
# becomes one special sequence. Inside comments a word-internal apostrophe
# becomes `’` and a "..." string containing \" becomes '...', so that the
# comment lexes as balanced quotes.

# The fences of grammar.md with the heading each one sits under.
def fences [lines: list<string>]: nothing -> list<record<heading: string, lines: list<string>>> {
    mut out = []
    mut heading = ""
    mut in_fence = false
    mut current: list<string> = []
    for line in $lines {
        if not $in_fence {
            if ($line | str starts-with "#") {
                $heading = $line | str replace -r '^#+\s*' ''
            } else if $line == "```ebnf" {
                $in_fence = true
                $current = []
            }
        } else if $line == "```" {
            $out ++= [{heading: $heading, lines: $current}]
            $in_fence = false
        } else {
            $current ++= [$line]
        }
    }
    $out
}

# The `commit \`x\`, DATE` of the first paragraph.
def provenance [text: string]: nothing -> record<commit: string, date: string> {
    let m = $text | parse -r 'commit `(?<commit>[0-9a-f]+)`, (?<date>\d{4}-\d{2}-\d{2})'
    match ($m | length) {
        0 => { commit: "unknown", date: "unknown" }
        _ => { commit: $m.0.commit, date: $m.0.date }
    }
}

# --- BNF -------------------------------------------------------------------

def bnf-text [blocks: list, prov: record]: nothing -> string {
    let header = [
        $"; The Nushell grammar as nu-parser accepts it \(nushell main ($prov.commit), ($prov.date))."
        "; Generated from grammar.md (the fences, in order) by tools/scripts/gen-grammar.nu."
        "; grammar.md carries the discussion, the coverage analysis and the meaning of"
        "; the annotations; this file is the same grammar in one piece. Regenerate it"
        "; from grammar.md rather than editing it directly."
        ";"
        "; Notation: <name> ::= alt | alt ; terminals in double quotes ; [ x ] optional ;"
        "; { x } zero or more ; ( x ) grouping ; 1*x one or more ; N*x exactly N ;"
        "; <words with spaces> is a class described in words, not a nonterminal ;"
        "; a line starting with ; is a comment. `; nu:` gives nu-parser's source,"
        "; `; here:` this crate's implementation."
    ]
    let body = $blocks | each {|b|
        let lines = if ($b.heading | str starts-with "0.") {
            $b.lines | each {|l| if ($l | is-empty) { ";" } else { $"; ($l)" } }
        } else {
            $b.lines
        }
        [""  $"; ==== ($b.heading) ====" ""] | append $lines
    } | flatten
    $header | append $body | str join "\n" | $in + "\n"
}

# --- EBNF ------------------------------------------------------------------

# The length of the "..." terminal starting at `chars[i]` (opening quote
# included): `"\""` is the double quote itself, otherwise the text runs to
# the next `"` (a lone `\` is not an escape: `"\"`). Null when unclosed.
def terminal-length [chars: list<string>, i: int]: nothing -> any {
    let rest = $chars | skip ($i + 1)
    if ($rest | first 3 | str join) == '\""' {
        return 4
    }
    let end = char-index $rest '"'
    if $end < 0 { null } else { $end + 2 }
}

# The index of the first `c` in `chars`, or -1.
def char-index [chars: list<string>, c: string]: nothing -> int {
    let hit = $chars | enumerate | where item == $c | get -o 0.index
    if $hit == null { -1 } else { $hit }
}

# Split a line at the `;` that starts its comment: one outside "..." and
# <...>, preceded by whitespace or at the start of the line.
def split-comment [line: string]: nothing -> record<body: string, comment: string> {
    let chars = $line | split chars
    let n = $chars | length
    mut i = 0
    mut angle = false
    while $i < $n {
        let c = $chars | get $i
        if $angle {
            if $c == ">" { $angle = false }
            $i += 1
        } else if $c == '"' {
            let len = terminal-length $chars $i
            if $len == null { break }
            $i += $len
        } else if $c == "<" {
            $angle = true
            $i += 1
        } else if $c == ";" and ($i == 0 or ($chars | get ($i - 1)) in [" " "\t"]) {
            let body = $chars | first $i | str join
            let comment = $chars | skip ($i + 1) | str join
            return { body: ($body | str trim), comment: ($comment | str trim) }
        } else {
            $i += 1
        }
    }
    { body: ($line | str trim), comment: "" }
}

# Split the notation part of a rule into tokens. `prose` is set when a word
# that is not part of the notation turns up: the rule is stated in words.
def tokenize [text: string]: nothing -> record<tokens: list<string>, prose: bool> {
    let chars = $text | split chars
    let n = $chars | length
    mut i = 0
    mut tokens: list<string> = []
    mut prose = false
    while $i < $n {
        let c = $chars | get $i
        if $c == " " or $c == "\t" {
            $i += 1
        } else if $c == '"' {
            let len = terminal-length $chars $i
            if $len == null {
                $prose = true
                break
            }
            let body = $chars | skip ($i + 1) | first ($len - 2) | str join
            $tokens ++= [(match $body { '\"' => "'\"'", '\\' => '"\"', _ => $'"($body)"' })]
            $i += $len
        } else if $c == "<" {
            let rest = $chars | skip ($i + 1)
            let end = char-index $rest ">"
            let body = $rest | first $end | str join
            if ($body | str contains " ") {
                $tokens ++= [$"? ($body) ?"]
            } else {
                $tokens ++= [($body | str replace -a "-" "_")]
            }
            $i += $end + 2
        } else if $c in ["[" "]" "{" "}" "(" ")" "|"] {
            $tokens ++= [$c]
            $i += 1
        } else if $c == "-" and ($chars | get -o ($i + 1)) == " " {
            $tokens ++= ["-"]
            $i += 1
        } else if ($c =~ '[0-9]') {
            # `1*x`, `N*x`, `1*6x`: a repetition count before the next token.
            let rest = $chars | skip $i | str join
            let m = $rest | parse -r '^(?<from>\d+)\*(?<to>\d*)'
            if ($m | is-empty) {
                $prose = true
                $i += 1
            } else {
                let from = $m.0.from
                let to = $m.0.to
                $tokens ++= [$"*($from):($to)"]
                $i += ($from | str length) + 1 + ($to | str length)
            }
        } else {
            $prose = true
            $i += 1
        }
    }
    { tokens: $tokens, prose: $prose }
}

# `unit` repeated as a marker `*from:to` says: `1*x` is `x, { x }`, `N*x` is
# `N * x`, `1*6x` is `x, 5 * [ x ]`.
def repeat [marker: string, unit: string]: nothing -> string {
    let parts = $marker | str substring 1.. | split row ":"
    let from = $parts.0 | into int
    if ($parts.1 | is-not-empty) {
        let to = $parts.1 | into int
        $"($unit), (($to - $from)) * [ ($unit) ]"
    } else if $from == 1 {
        $"($unit), { ($unit) }"
    } else {
        $"($from) * ($unit)"
    }
}

# Tokens joined the ISO way: `,` between items, nothing after an opener,
# before a closer, or around `|` and `-`.
def join-items [items: list<string>]: nothing -> string {
    mut text = ""
    mut prev = ""
    for item in $items {
        let glue = if ($prev | is-empty) {
            ""
        } else if $prev in ["[" "{" "(" "|" "-"] or $item in ["]" "}" ")" "|" "-"] {
            " "
        } else {
            ", "
        }
        $text = $text + $glue + $item
        $prev = $item
    }
    $text
}

# Render tokens in ISO EBNF. A repetition marker applies to the next token,
# or to the whole group when a `(` follows it.
def render-tokens [tokens: list<string>]: nothing -> string {
    mut out: list<string> = []
    mut rep = ""
    mut i = 0
    let n = $tokens | length
    while $i < $n {
        let t = $tokens | get $i
        $i += 1
        if ($t | str starts-with "*") {
            $rep = $t
            continue
        }
        if ($rep | is-empty) {
            $out ++= [$t]
            continue
        }
        if $t == "(" {
            # The group up to the matching `)` is the unit.
            mut depth = 1
            mut group = ["("]
            while $depth > 0 and $i < $n {
                let g = $tokens | get $i
                $i += 1
                $depth += (match $g { "(" => 1, ")" => -1, _ => 0 })
                $group ++= [$g]
            }
            $out ++= [(repeat $rep (join-items $group))]
        } else {
            $out ++= [(repeat $rep $t)]
        }
        $rep = ""
    }
    join-items $out
}

# Comment text made safe for an EBNF lexer: outside "..." a word-internal
# apostrophe becomes `’`; a "..." string holding \" becomes '...'.
def comment-text [text: string]: nothing -> string {
    let chars = $text | split chars
    let n = $chars | length
    mut i = 0
    mut out = ""
    while $i < $n {
        let c = $chars | get $i
        if $c == '"' {
            let len = terminal-length $chars $i
            if $len == null {
                $out = $out + ($chars | skip $i | str join)
                break
            }
            let body = $chars | skip ($i + 1) | first ($len - 2) | str join
            $i += $len
            if $body == '\"' {
                $out = $out + "'\"'"
                continue
            }
            # `"a\"b"`: a quote inside the string, so the string takes single quotes.
            if ($body | str contains '\"') {
                $out = $out + "'" + ($body | str replace -a '\"' '"') + "'"
                continue
            }
            # `"a\"` closed the string early: read on to the real closing quote.
            if ($body | str ends-with '\') and $len > 3 {
                let rest = $chars | skip $i
                let close = char-index $rest '"'
                if $close >= 0 {
                    let more = $rest | first $close | str join
                    let inner = $body | str substring 0..<(($body | str length) - 1) | str replace -a '\"' '"'
                    $out = $out + "'" + $inner + '"' + $more + "'"
                    $i += $close + 1
                    continue
                }
            }
            $out = $out + '"' + $body + '"'
        } else if $c == "'" and $i > 0 and $i + 1 < $n and ($chars | get ($i - 1)) =~ '\w' and ($chars | get ($i + 1)) =~ '\w' {
            $out = $out + "’"
            $i += 1
        } else {
            $out = $out + $c
            $i += 1
        }
    }
    $out
}

# The lines of a rule (its first line and the indented continuations that
# follow) rendered in ISO EBNF, comments kept where they were.
def render-rule [name: string, lines: list<string>]: nothing -> list<string> {
    let ident = $name | str replace -a "-" "_"
    let parts = $lines | each {|l| split-comment $l }
    let joined = $parts | get body | where ($it | is-not-empty) | str join " "
    let toks = tokenize $joined
    if $toks.prose {
        let comments = $parts | get comment | where ($it | is-not-empty)
        let rule = $"($ident) = ? ($joined) ? ;"
        return ([$rule] | append (comment-block $comments))
    }
    let indent = "" | fill -c " " -w (($ident | str length) + 3)
    # Render the content lines first, to know where a line break needs a `,`.
    let rendered = $parts | where ($it.body | is-not-empty) | each {|p|
        let tokens = (tokenize $p.body).tokens
        { text: (render-tokens $tokens), first: ($tokens | first), last: ($tokens | last), comment: $p.comment }
    }
    let count = $rendered | length
    mut out: list<string> = []
    mut seen = 0
    for p in $parts {
        if ($p.body | is-empty) {
            $out ++= [$"($indent)\(* (comment-text $p.comment) *)"]
            continue
        }
        let r = $rendered | get $seen
        let lead = if $seen == 0 { $"($ident) = " } else { $indent }
        let next = $rendered | get -o ($seen + 1)
        let continues = $next != null and $next.first != "|" and not ($r.last in ["(" "[" "{" "|"])
        let close = if $seen == $count - 1 { " ;" } else if $continues { "," } else { "" }
        let comment = if ($r.comment | is-empty) { "" } else { $"  \(* (comment-text $r.comment) *)" }
        $out ++= [$"($lead)($r.text)($close)($comment)"]
        $seen += 1
    }
    $out
}

# Consecutive comment lines as one `(* ... *)` block, each line trimmed
# unless `raw` (the notation section keeps its alignment).
def comment-block [comments: list<string>, --raw]: nothing -> list<string> {
    if ($comments | is-empty) {
        return []
    }
    let safe = $comments | each {|c| comment-text (if $raw { $c | str trim -r } else { $c | str trim }) }
    if ($safe | length) == 1 {
        return [$"\(* ($safe.0) *)"]
    }
    let first = $"\(* ($safe.0)"
    let middle = $safe | skip 1 | drop 1 | each {|c| $"   ($c)" }
    let last = $"   ($safe | last) *)"
    [$first] | append $middle | append [$last]
}

# One fence in ISO EBNF.
def ebnf-block [lines: list<string>]: nothing -> list<string> {
    mut out: list<string> = []
    mut comments: list<string> = []
    mut rule_name = ""
    mut rule_lines: list<string> = []
    for line in ($lines | append [""]) {
        let is_comment = ($line | str starts-with ";")
        let is_continuation = ($line | str starts-with " ") and ($rule_name | is-not-empty)
        if $is_continuation {
            $rule_lines ++= [$line]
            continue
        }
        if ($rule_name | is-not-empty) {
            $out ++= (render-rule $rule_name $rule_lines)
            $rule_name = ""
            $rule_lines = []
        }
        if $is_comment {
            $comments ++= [($line | str substring 1..)]
            continue
        }
        if ($comments | is-not-empty) {
            $out ++= (comment-block $comments)
            $comments = []
        }
        let head = $line | parse -r '^<(?<name>[a-z0-9-]+)>\s*::=(?<rest>.*)$'
        if ($head | is-not-empty) {
            $rule_name = $head.0.name
            $rule_lines = [$head.0.rest]
        } else if ($line | is-empty) {
            if ($out | is-not-empty) and (($out | last) | is-not-empty) {
                $out ++= [""]
            }
        } else {
            $out ++= [$"\(* (comment-text $line) *)"]
        }
    }
    if ($out | is-not-empty) and (($out | last) | is-empty) {
        $out | drop 1
    } else {
        $out
    }
}

def ebnf-text [blocks: list, prov: record]: nothing -> string {
    let header = [
        $"\(* The Nushell grammar as nu-parser accepts it \(nushell main ($prov.commit), ($prov.date)). *)"
        "(* Generated from grammar.md (the fences, in order) by tools/scripts/gen-grammar.nu. *)"
        "(* grammar.md carries the discussion, the coverage analysis and the meaning of *)"
        "(* the annotations; this file is the same grammar in one piece. Regenerate it *)"
        "(* from grammar.md rather than editing it directly. *)"
        "(* *)"
        "(* ISO 14977 style: name = definition ; with , concatenation, | alternatives, *)"
        "(* [ x ] optional, { x } repetition, N * x exactly N, \"x\" / 'x' terminals, *)"
        "(* ? ... ? special sequences for rules stated in prose; comments are bracketed *)"
        "(* by a paren-star pair, as in Pascal. *)"
        "(* `nu:` names the nu-parser source, `here:` the implementation in this crate. *)"
    ]
    let body = $blocks | each {|b|
        let lines = if ($b.heading | str starts-with "0.") {
            comment-block --raw ($b.lines | where ($it | is-not-empty))
        } else {
            ebnf-block $b.lines
        }
        ["" $"\(* ==== ($b.heading) ==== *)" ""] | append $lines
    } | flatten
    $header | append $body | str join "\n" | $in + "\n"
}

# Rewrite the generated grammar files, or with --check report whether they are current.
def main [
    --root: path    # the repository root (default: the parent of this script's directory)
    --check         # do not write; exit 1 when a generated file is stale
] {
    let root = $root | default ($env.FILE_PWD | path join ../.. | path expand)
    let md_path = $root | path join grammar/grammar.md
    let md = open --raw $md_path
    let prov = provenance $md
    let blocks = fences ($md | lines)
    let outputs = [
        [path text];
        [($root | path join grammar/grammar.bnf) (bnf-text $blocks $prov)]
        [($root | path join grammar/grammar.ebnf) (ebnf-text $blocks $prov)]
    ]
    mut stale: list<string> = []
    for o in $outputs {
        let current = if ($o.path | path exists) { open --raw $o.path } else { "" }
        if $current != $o.text {
            $stale ++= [$o.path]
            if not $check {
                $o.text | save -f $o.path
            }
        }
    }
    if $check and ($stale | is-not-empty) {
        error make { msg: $"stale generated grammar: ($stale | str join ', '); run nu tools/scripts/gen-grammar.nu" }
    }
    print $"($blocks | length) fences; (if $check { 'checked' } else { 'wrote' }) grammar.bnf and grammar.ebnf"
}
