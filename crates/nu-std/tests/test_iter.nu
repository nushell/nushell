use std *

export def test_iter_cmd [] {
    test_iter_intersperse
    test_iter_find
}

def test_iter_find [] {
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

def test_iter_intersperse [] {
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
