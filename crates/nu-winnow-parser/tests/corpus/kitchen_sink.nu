#!/usr/bin/env nu
# A file exercising most of the Nushell grammar in one place.
# It is parsed by the test-suite and checked for span consistency.

use std/log
use std [assert]
use std *
const VERSION = "1.0"
export-env { $env.KITCHEN = "sink" }

# Documentation for greet.
@example "say hi" { greet world } --result "hello world"
@search-terms hello salutation
export def greet [
    name: string = "world"  # who to greet
    --loud(-l)              # shout
    --times (-t): int = 1
    ...rest: any
]: nothing -> string {
    let greeting = $"hello ($name)"
    if $loud { $greeting | str uppercase } else { $greeting }
}

def --env "set up" [] { $env.SET_UP = true }
def --wrapped wrap [...args] { ^echo ...$args }
extern ext [--flag: string, positional?: int]
alias ll = ls -la
module inner {
    export def f []: int -> int { $in + 1 }
    export const c = 3
}

let list = [1, 2 3
  4 # inline comment
  ...[5 6]
]
mut rec = {a: 1, "b c": [2], d: {e: 0x[00]}, ...{f: 1.5}}
$rec.a += 10
$rec.d.e = 0x[de ad be ef]
let table = [[name size]; ["a" 1kb] ["b" 2.5mb]]
let range = 0..<10
let step = 1..3..10
let when = 2024-01-02T03:04:05Z
let dur = 1.5hr + 30min
let raw = r#'raw "string" with 'quotes''#
let interp = $'single (1 + 1)' + $"double \(escaped\) ($list | length)"
let math = (1 + 2) * 3 ** 2 ** 1 // 4 mod 3 - -1
let logic = not ($math > 1) and $list.0? in [1 2] or $raw =~ 'raw' xor true
let bits = 1 bit-shl 2 bit-or 4 bit-and 7 bit-xor 1 bit-shr 0
let cmp = "a" starts-with "a" and "b" not-ends-with "c" and 1 not-in [2] and [1] has 1
let closure = {|x: int, y?| $x + ($y | default 0) }
let block_like = { || $in }
let nested = {|| {|| 1 } }
let sub = (
    ls
    | where size > 1kb and name =~ '\.nu$'
    | sort-by modified --reverse
    | first 3
)
let path = $env.PWD | path join "x"
let cellpath = $.a.b?.c!

for x in $list { print $x }
for y: int in 1..3 { continue }
while $rec.a < 20 { $rec.a += 1; if $rec.a == 15 { break } }
loop { break }

match $rec {
    {a: 1} => "one",
    {a: $n} if $n > 5 => $"big ($n)"
    [$first, ..$others] => { print $first; $others }
    1 | 2 | 3 => (print "small")
    "str" | 'other' => {|| 1 }
    _ => null
}

try { error make {msg: "boom"} } catch { |e| log error $e.msg } finally { print done }

def rows [] {
    ls | where type == file | where {|f| $f.size > 1kb }
}

FOO=bar BAZ=$VERSION ^env | lines | where $it starts-with "FOO"
^echo hello o> /dev/null e> /dev/null
^echo build o+e>| lines | length
^ls e>| length

ls
# comment between
| get name
| each { |n|
    # closure body comment
    $n | str uppercase
}
| str join ", "

if ($list | is-empty) {
    print "empty"
} else if ($list | length) > 3 {
    print "long"
} else {
    print "short"
}

def "multi word" [] { "multi" }
multi word | print
hide-env --ignore-errors FOO
overlay list
1 + 1 | into string; print (2 * 2)
