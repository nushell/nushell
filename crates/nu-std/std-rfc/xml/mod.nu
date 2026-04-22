# Custom commands to read, change and create XML data in format supported by the `to xml` and `from xml` commands

use std-rfc/iter [ recurse ]

# def __children_0 []: list<record> -> list<list<record>> {
#     where ($it.content | describe --detailed).type == list
#     | get content
# }

# 12x faster than `__children_0` in a benchmark
def __children_1 []: list<record> -> list<list<record>> {
    each {
        match $in.content {
            [..$children] => $children
        }
    }
}

def children []: list<record> -> list<record> {
    __children_1 | flatten
}

def descendant-or-self []: list<record<content: list<record>>> -> list<record>  {
    recurse { __children_1 }
    | get item
    | flatten
}

def make-list []: any -> list {
    match $in {
        [..$xs] => $xs
        $x => [$x]
    }
}

def pipeline [meta: record]: list<oneof<cell-path, string, int, closure, list>> -> closure {
    let steps = each {|e|
        if ($e | describe) == "cell-path" {
            $e | split cell-path | get value
        } else {
            $e | make-list  # make sure it's a list so `flatten` behaves in a predictable manner
        }
    }
    | flatten

    if ($steps | is-empty) {
        error make {
            msg: 'Missing query'
            label: {
                text: 'Requires a query'
                span: $meta.span
            }
        }
    }

    $steps
    | reduce --fold {|| } {|step, prev|
        match ($step | describe) {
            "string" => {
                match $step {
                    "*" => {|| do $prev | children }
                    "**" => {|| do $prev | descendant-or-self }
                    $tag => {|| do $prev | children | where tag == $tag }
                }
            }
            "int" => {|| do $prev | select $step }
            "closure" => {|| do $prev | where $step }
            $type => {
                let step_span = (metadata $step).span
                error make {
                    code: "nu::shell::type_mismatch"
                    msg: 'Incorrect query component type'
                    label: {
                        text: $'expected string, int or closure; got ($type)'
                        span: $step_span
                    }
                }
            }
        }
    }
}

# Get all xml entries matching simple xpath-inspired query
# 
# The query can include:
#
# - cell-path:
#   -  `*`: Get all children. (`child::node()`)
#   - `**`: Get all descendants. (`descendant-or-self::node()`)
#   - string: Get all children with specified name. (`A` == `child::A`)
#   - int: Select n-th among nodes selected by previous path. (0-indexed)
#
#   Example: `A.**.B.*.0` == `A//B/*[1]`
#
# - closure:
#   Predicate. Select all entries for which predicate returns true.
export def xaccess [...query: oneof<cell-path, closure, list<oneof<int, string, closure>>>] {
    let doc = $in | make-list
    let filter = $query | pipeline (metadata $query)

    [
        [tag, attributes, content];
        [null, null, $doc]
    ]
    | do $filter
}
