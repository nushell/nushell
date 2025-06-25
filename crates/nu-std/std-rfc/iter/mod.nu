def cell-path-join []: list<cell-path> -> cell-path {
    each {|e| try { split cell-path } catch { $e } }
    | flatten
    | into cell-path
}

def add-parent [parent: cell-path]: table<path: cell-path> -> table<path: cell-path> {
    update path { [$parent, $in] | cell-path-join }
}

def get-children []: [any -> table<path: cell-path, item: any>] {
    let val = $in
    match ($val | describe --detailed).type {
        "record" => { $val | transpose path item }
        "list" => { $val | enumerate | rename path item }
        _ => { return [] }
    }
}

def get-children-at [path: cell-path]: [any -> table<path: cell-path, item: any>] {
    let x = try { get $path } catch { return [] }

    if ($x | describe --detailed).type == "list" {
        $x | get-children | add-parent $path
    } else {
        [{
            path: $path
            item: $x
        }]
    }
}

# Recursively descend a nested value, returning each value along with its path.
#
# Recursively descends its input, producing all values as a stream, along with
# the cell-paths to access those values.
#
# If a cell-path is provided as argument, rather than traversing all children,
# only the given cell-path is followed. The cell-path is evaluated at each level,
# relative to the parent element.
#
# If a closure is provided, it will be used to get children from parent values.
# The closure can have a variety of return types, each one in the list being
# coerced to the next type:
#  - list<any>
#  - table<item: any>
#  - table<item: any, path: any>
# `path` is used to construct the full path of an item, being concatenated to
# the parent item's path. If a child item does not have a `path` field, its
# path defaults to `<closure>`
@example "Access each possible path in a value" {
    {
        "foo": {
            "egg": "X"
            "spam": "Y"
        }
        "bar": {
            "quox": ["A" "B"]
        }
    }
    | recurse
    | update item { to nuon }
} --result [
    [path, item];
    [ ($.),           r#'{foo: {egg: X, spam: Y}, bar: {quox: [A, B]}}'# ],
    [ ($.foo),        r#'{egg: X, spam: Y}'# ],
    [ ($.bar),        r#'{quox: [A, B]}'# ],
    [ ($.foo.egg),    r#'X'# ],
    [ ($.foo.spam),   r#'Y'# ],
    [ ($.bar.quox),   r#'[A, B]'# ],
    [ ($.bar.quox.0), r#'A'# ],
    [ ($.bar.quox.1), r#'B'# ]
]
@example "Recurse example from `jq`'s manpage" {
    {"name": "/", "children": [
        {"name": "/bin", "children": [
            {"name": "/bin/ls", "children": []},
            {"name": "/bin/sh", "children": []}]},
        {"name": "/home", "children": [
            {"name": "/home/stephen", "children": [
                {"name": "/home/stephen/jq", "children": []}]}]}]}
    | recurse children
    | get item.name
} --result [/, /bin, /home, /bin/ls, /bin/sh, /home/stephen, /home/stephen/jq]
@example "Recurse example from `jq`'s manpage, using depth-first traversal like `jq`" {
    {"name": "/", "children": [
        {"name": "/bin", "children": [
            {"name": "/bin/ls", "children": []},
            {"name": "/bin/sh", "children": []}]},
        {"name": "/home", "children": [
            {"name": "/home/stephen", "children": [
                {"name": "/home/stephen/jq", "children": []}]}]}]}
    | recurse children --depth-first
    | get item.name
} --result [/, /bin, /bin/ls, /bin/sh, /home, /home/stephen, /home/stephen/jq]
@example '"Recurse" using a closure' {
    2
    | recurse { ({path: square item: ($in * $in)}) }
    | take while { $in.item < 100 }
} --result [
    [path, item];
    [$., 2],
    [$.square, 4],
    [$.square.square, 16]
]
@search-terms jq ".." nested
export def recurse [
    get_children?: oneof<cell-path, closure> # Specify how to get children from parent value.
    --depth-first # Descend depth-first rather than breadth first
]: [any -> list<any>] {
    let descend = match ($get_children | describe --detailed).type {
        "nothing" => {
            {|| get-children }
        }
        "cell-path" | "string" | "int" => {
            {|| get-children-at $get_children }
        }
        "closure" => {
            {|parent|
                let output = try {
                    $parent | do $get_children $parent
                } catch {
                    return []
                }
                | append []
                let has_item = try { $output | get item; true } catch { false }

                $output
                | if not $has_item { wrap item } else { }
                | default "<closure>" path
            }
        }
        $type => {
            error make {
                msg: "Type mismatch."
                label: {
                    text: $"Cannot get child values using a ($type)"
                    span: (metadata $get_children).span
                }
                help: "Try using a cell-path or a closure."
            }
        }
    }

    let fn = if $depth_first {
        {|stack|
            match $stack {
                [] => { {} }
                [$head, ..$tail] => {
                    let children = $head.item | do $descend $head.item | add-parent $head.path
                    {
                        out: $head,
                        next: ($tail | prepend $children),
                    }
                }
            }
        }
    } else {
        {|out|
            let children = $out
            | each {|e| $e.item | do $descend $e.item | add-parent $e.path }
            | flatten

            if ($children | is-not-empty) {
                {out: $out, next: $children}
            } else {
                {out: $out}
            }
        }
    }

    generate $fn [{path: ($.), item: ($in) }]
    | if not $depth_first { flatten } else { }
}

# Helper for `only` errors
def only-error [msg: string, meta: record, label: string]: nothing -> error {
  error make {
    msg: $msg,
    label: {
      text: $label,
      span: $meta.span,
    }
  }
}

# Get the only element of a list or table, ensuring it exists and there are no extra elements.
#
# Similar to `first` with no arguments, but errors if there are additional
# items when there should only be one item. This can help avoid issues when more
# than one row than expected matches some criteria.
#
# This command is useful when chained with `where` to ensure that only one row
# meets the given condition.
#
# If a cell path is provided as an argument, it will be accessed after the first
# element. For example, `only foo` is roughly equivalent to `get 0.foo`, with
# the guarantee that there are no additional elements.
#
# Note that this command currently collects streams.
@search-terms first single
@category filters
@example "Get the only item in a list, ensuring it exists and there's no additional items" --result 5 {
  [5] | only
}
@example "Get the `name` column of the only row in a table" --result "foo" {
  [{name: foo, id: 5}] | only name
}
@example "Get the modification time of the file named foo.txt" {
  ls | where name == "foo.txt" | only modified
}
export def only [
  cell_path?: cell-path # The cell path to access within the only element.
]: [table -> any, list -> any] {
  let pipe = {in: $in, meta: (metadata $in)}
  let path = [0 $cell_path] | cell-path-join
  match ($pipe.in | length) {
    0 => (only-error "expected non-empty table/list" $pipe.meta "empty")
    1 => ($pipe.in | get $path)
    _ => (only-error "expected only one element in table/list" $pipe.meta "has more than one element")
  }
}
