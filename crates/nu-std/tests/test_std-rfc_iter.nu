use std/assert
use std/testing *
use std-rfc/iter *

@test
def recurse-example-basic [] {
    let out = {
        "foo": {
            "egg": "X"
            "spam": "Y"
        }
        "bar": {
            "quox": ["A" "B"]
        }
    }
    | recurse

    let expected = [
        [path, item];
        [ ($.),           {foo: {egg: X, spam: Y}, bar: {quox: [A, B]}} ],
        [ ($.foo),        {egg: X, spam: Y} ],
        [ ($.bar),        {quox: [A, B]} ],
        [ ($.foo.egg),    X ],
        [ ($.foo.spam),   Y ],
        [ ($.bar.quox),   [A, B] ],
        [ ($.bar.quox.0), A ],
        [ ($.bar.quox.1), B ]
    ]

    assert equal $out $expected
}

@test
def recurse-example-jq [] {
    let out = {"name": "/", "children": [
        {"name": "/bin", "children": [
            {"name": "/bin/ls", "children": []},
            {"name": "/bin/sh", "children": []}]},
        {"name": "/home", "children": [
            {"name": "/home/stephen", "children": [
                {"name": "/home/stephen/jq", "children": []}]}]}]}
    | recurse children
    | get item.name

    let expected = [/, /bin, /home, /bin/ls, /bin/sh, /home/stephen, /home/stephen/jq]

    assert equal $out $expected
}

@test
def recurse-example-jq-depth-first [] {
    let out = {"name": "/", "children": [
        {"name": "/bin", "children": [
            {"name": "/bin/ls", "children": []},
            {"name": "/bin/sh", "children": []}]},
        {"name": "/home", "children": [
            {"name": "/home/stephen", "children": [
                {"name": "/home/stephen/jq", "children": []}]}]}]}
    | recurse children --depth-first
    | get item.name

    let expected = [/, /bin, /bin/ls, /bin/sh, /home, /home/stephen, /home/stephen/jq]

    assert equal $out $expected
}

@test
def recurse-example-closure [] {
    let out = 2
    | recurse { ({path: square item: ($in * $in)}) }
    | take while { $in.item < 100 }

    let expected = [
        [path, item];
        [$., 2],
        [$.square, 4],
        [$.square.square, 16]
    ]

    assert equal $out $expected
}
