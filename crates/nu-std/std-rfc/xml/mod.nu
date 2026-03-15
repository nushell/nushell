# Custom commands to read, change and create XML data in format supported by the `to xml` and `from xml` commands

use std-rfc/iter [ recurse ]

def children []: list<record> -> list<record> {
    where ($it.content | describe --detailed).type == list
    | get content
    | flatten
}

def descendant-or-self []: list<record<content: list<record>>> -> list<record>  {
    recurse {
        where ($it.content | describe --detailed).type == list
        | get content
    }
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
            msg: 'Empty path provided'
            label: {
                text: 'Use a non-empty list of path steps'
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
                    msg: $'Incorrect path step type ($type)'
                    label: {
                        text: 'Use a string or int as a step'
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
