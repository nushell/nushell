use std *

export def test_iter_find [] {
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

export def test_iter_intersperse [] {
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

export def test_iter_scan [] {
    let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y} -n)
    assert equal $scanned [1, 3, 6]

    let scanned = ([1 2 3] | iter scan 0 {|x, y| $x + $y})
    assert equal $scanned [0, 1, 3, 6]

    let scanned = ([a b c d] | iter scan "" {|x, y| [$x, $y] | str join} -n)
    assert equal $scanned ["a" "ab" "abc" "abcd"]
}

export def test_iter_filter_map [] {
    let res = ([2 5 "4" 7] | iter filter-map {|it| $it ** 2})
    assert equal $res [4 25 49]

    let res = (
        ["3" "42" "69" "n" "x" ""] 
        | iter filter-map {|it| $it | into int}
        )
    assert equal $res [3 42 69]
}
