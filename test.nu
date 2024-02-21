# Glob Torture Test

## This is a stress test for globbing. 
# 1. Put the file to an empty directory.
# 2. Edit file to remove markdown.
# 3. Run `nu glob_torture.nu`
# 4. Fix TODOs


use std assert

def setup [] {
    'a*c' o> 'a*c.txt'
    'abc' o> 'abc.txt'
    'azc' o> 'azc.txt'
    'def main [...rest] { $rest | str join " " }' o> 'test.nu'
}

def cleanup [] {
    rm *.txt
    rm test.nu
}

def test-with-setup [cmd: closure, expected, tag: string] {
    setup
    let actual = do $cmd
    try {
        assert equal $actual $expected
    } catch {
        print $'Assertion failed! tag: ($tag)'
        print $'  Expected: ($expected)'
        print $'  Got:      ($actual)'
    }
    cleanup
}

## Test setup and cleanup, already tests some globbing

setup
assert equal (ls *.txt | get name) [a*c.txt abc.txt azc.txt]

cleanup
assert equal (do -i { ls *.txt }) null

## Test globbing with builtin commands (open)

let cmd = { open a*c.txt }
let expected = [a*c abc azc]
test-with-setup $cmd $expected "builtin 1"

let cmd = { open 'a*c.txt' }
let expected = 'a*c'
test-with-setup $cmd $expected "builtin 2"

let cmd = {
    let gvar = 'a*c.txt'
    open $gvar
}
let expected = 'a*c'
test-with-setup $cmd $expected "builtin 3"

let cmd = {
    let gvar = 'a*c.txt'
    # call with glob expansion
    open ($gvar | into glob)
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "builtin 4"

let cmd = {
    let gvar: glob = 'a*c.txt'
    # Call open with glob expansion
    open $gvar
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "builtin 5"

let cmd = {
    let gvar: glob = 'a*c.txt'
    # Call open without glob expansion
    open ($gvar | into string)
}
let expected = 'a*c'
test-with-setup $cmd $expected "builtin 6"

# Test globbing with custom commands

def glob-test [g: glob] { open $g }

let cmd = { glob-test a*c.txt }
let expected = [a*c abc azc]
test-with-setup $cmd $expected "custom 1"

let cmd = { glob-test 'a*c.txt' }
let expected = 'a*c'
test-with-setup $cmd $expected "custom 2"

let cmd = {
    let gvar = 'a*c.txt'
    # Call glob-test with glob expansion
    glob-test $gvar
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "custom 3"


let cmd = {
    let gvar: glob = 'a*c.txt'
    # Call glob-test with glob expansion
    glob-test $gvar
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "custom 5"

# NOTE: we don't have a way to call glob-test without glob expansion
# because `g` is already defined as a glob
# let cmd = {
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call glob-test without glob expansion
#     # currently we don't have a way to do this, because we already
#     # make sure that the argument type is a glob
# }
# let expected = [a*c]
# test-with-setup $cmd $expected "custom 6"


# NOTE: we don't have a way to call glob-test without glob expansion
# because `g` is already defined as a glob
# let cmd = {
#     let gvar = 'a*c.txt'
#     # TODO: Call glob-test without glob expansion
#     # currently we don't have a way to do this, because we already
#     # make sure that the argument type is a glob
# }
# let expected = [a*c]
# test-with-setup $cmd $expected "custom 4"

# Test globbing with custom command, the argument type is string
def glob-test-2 [g: string] { open $g }

let cmd = { glob-test-2 a*c.txt }
let expected = "a*c"
test-with-setup $cmd $expected "custom 21"

let cmd = { glob-test-2 'a*c.txt' }
let expected = 'a*c'
test-with-setup $cmd $expected "custom 22"

let cmd = {
    let gvar = 'a*c.txt'
    # Call glob-test-2 without glob expansion
    glob-test-2 $gvar
}
let expected = "a*c"
test-with-setup $cmd $expected "custom 24"

# NOTE: we don't have a way to call glob-test-2 with glob expansion
# because `g` is already defined as a string
# let cmd = {
#     let gvar = 'a*c.txt'
#     # TODO: Call glob-test-2 with glob expansion
#     # Hmm, I don't think nushell needs to support this, because argument `g` is a string
# }
# let expected = "a*c"
# test-with-setup $cmd $expected "custom 23"
#
#
# NOTE: we don't have a way to call glob-test-2 with glob expansion
# because `g` is already defined as a string
# let cmd = {
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call glob-test-2 with glob expansion
#     # Hmm, I don't think nushell needs to support this, because argument `g` is a string
# }
# let expected = [a*c]
# test-with-setup $cmd $expected "custom 25"

let cmd = {
    let gvar: glob = 'a*c.txt'
    # Call glob-test-2 without glob expansion
    glob-test-2 $gvar
}
let expected = "a*c"
test-with-setup $cmd $expected "custom 26"

# Test globbing with pipeline input
def glob-test-3 [] { open $in }

let cmd = { "a*c.txt" | glob-test-3 }
let expected = "a*c"
test-with-setup $cmd $expected "custom 31"

let cmd = { 'a*c.txt' | glob-test-3 }
let expected = 'a*c'
test-with-setup $cmd $expected "custom 32"

let cmd = {
    let gvar = 'a*c.txt'
    # Call glob-test-3 with glob expansion
    $gvar | into glob | glob-test-3
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "custom 33"

let gvar = 'a*c.txt'
let cmd = {
    let gvar = 'a*c.txt'
    # Call glob-test-3 without glob expansion
    $gvar | glob-test-3
}
let expected = "a*c"
test-with-setup $cmd $expected "custom 34"

let cmd = {
    let gvar: glob = 'a*c.txt'
    # TODO: Call glob-test-3 with glob expansion
    $gvar | into glob | glob-test-3
}
let expected = [a*c abc azc]
test-with-setup $cmd $expected "custom 35"

let cmd = {
    let gvar: glob = 'a*c.txt'
    # TODO: Call glob-test-3 without glob expansion
    $gvar | into string | glob-test-3
}
let expected = "a*c"
test-with-setup $cmd $expected "custom 36"



## TODO: Test globbing with external commands
#
# let cmd = { ^$nu.current-exe test.nu a*c.txt}
# let expected = "a*c.txt abc.txt azc.txt\n"
# test-with-setup $cmd $expected "external 1"
#
# let cmd = { ^$nu.current-exe test.nu 'a*c.txt'}
# let expected = "a*c.txt\n"
# test-with-setup $cmd $expected "external 2"
#
# let cmd = {||
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call ^$nu.current-exe test.nu with glob expansion
# }
# let expected = [a*c abc azc]
# test-with-setup $cmd $expected "external 3"
#
# let cmd = {||
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call ^$nu.current-exe test.nu without glob expansion
# }
# let expected = [a*c]
# test-with-setup $cmd $expected "external 4"
#
#
# let cmd = {||
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call ^$nu.current-exe test.nu with glob expansion
# }
# let expected = [a*c abc azc]
# test-with-setup $cmd $expected "external 5"
#
# let cmd = {||
#     let gvar: glob = 'a*c.txt'
#     # TODO: Call ^$nu.current-exe test.nu without glob expansion
# }
# let expected = [a*c]
# test-with-setup $cmd $expected "external 6"
