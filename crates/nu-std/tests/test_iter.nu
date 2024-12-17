use std *
use std/assert

#[test]
def iter_find [] {
    let hastack1 = [1 2 3 4 5 6 7]
    let hastack2 = [nushell rust shell iter std]
    let hastack3 = [nu 69 2023-04-20 "std"]

    let res = ($hastack1 | iter find {|it| $it mod 2 == 0})
    assert equal $res 2

    let res = ($hastack2 | iter find {|it| $it starts-with 's'})
    assert equal $res 'shell'

    let res = ($hastack2 | iter find {|it| ($it | length) == 50})
    assert equal $res null

    let res = ($hastack3 | iter find {|it| (it | describe) == filesize})
    assert equal $res null
}

#[test]
def iter_intersperse [] {
    let res = ([1 2 3 4] | iter intersperse 0)
    assert equal $res [1 0 2 0 3 0 4]

    let res = ([] | iter intersperse x)
    assert equal $res []

    let res = ([1] | iter intersperse 5)
    assert equal $res [1]

    let res = ([a b c d e] | iter intersperse 5)
    assert equal $res [a 5 b 5 c 5 d 5 e]

    let res = (1..4 | iter intersperse 0)
    assert equal $res [1 0 2 0 3 0 4]

    let res = (4 | iter intersperse 1)
    assert equal $res [4]
}

#[test]
def iter_scan [] {
    let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y} -n)
    assert equal $scanned [1, 3, 6]

    let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y})
    assert equal $scanned [0, 1, 3, 6]

    let scanned = ([a b c d] | iter scan "" {|it, acc| [$acc, $it] | str join} -n)
    assert equal $scanned ["a" "ab" "abc" "abcd"]

    let scanned = ([a b c d] | iter scan "" {|it, acc| append $it | str join} -n)
    assert equal $scanned ["a" "ab" "abc" "abcd"]
}

#[test]
def iter_filter_map [] {
    let res = ([2 5 "4" 7] | iter filter-map {|it| $it ** 2})
    assert equal $res [4 25 49]

    let res = (
        ["3" "42" "69" "n" "x" ""]
        | iter filter-map {|it| $it | into int}
        )
    assert equal $res [3 42 69]
}

#[test]
def iter_find_index [] {
    let res = (
         ["iter", "abc", "shell", "around", "nushell", "std"]
         | iter find-index {|x| $x starts-with 's'}
    )
    assert equal $res 2

    let is_even = {|x| $x mod 2 == 0}
    let res = ([3 5 13 91] | iter find-index $is_even)
    assert equal $res (-1)

    let res = (42 | iter find-index {|x| $x == 42})
    assert equal $res 0
}

#[test]
def iter_zip_with [] {
    let res = (
        [1 2 3] | iter zip-with [2 3 4] {|a, b| $a + $b }
    )

    assert equal $res [3 5 7]

    let res = (42 | iter zip-with [1 2 3] {|a, b| $a // $b})
    assert equal $res [42]

    let res = (2..5 | iter zip-with 4 {|a, b| $a * $b})
    assert equal $res [8]

    let res = (
        [[name repo]; [rust github] [haskell gitlab]]
        | iter zip-with 1.. {|data, num|
            { name: $data.name, repo: $data.repo position: $num }
        }
    )
    assert equal $res [
        [name    repo    position];
        [rust    github  1]
        [haskell gitlab  2]
    ]
}

#[test]
def iter_flat_map [] {
    let res = (
        [[1 2 3] [2 3 4] [5 6 7]] | iter flat-map {|it| $it | math sum}
    )
    assert equal $res [6 9 18]

    let res = ([1 2 3] | iter flat-map {|it| $it + ($it * 10)})
    assert equal $res [11 22 33]
}

#[test]
def iter_zip_into_record [] {
    let headers = [name repo position]
    let values = [rust github 1]

    let res = (
        $headers | iter zip-into-record $values
    )

    assert equal $res [
        [name    repo    position];
        [rust    github  1]
    ]
}
