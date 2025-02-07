def aggregate-default-ops [] {
    {
        min: {math min},
        avg: {math avg},
        max: {math max},
        sum: {math sum},
    }
}

def aggregate-col-name [col: cell-path, op_name: string]: [nothing -> string] {
    $col | split cell-path | get value | str join "." | $"($in)_($op_name)"
}

def get-item-with-error [
    col: cell-path,
    opts: record<span: record<start: int, end: int>, items: bool>
]: [table -> any] {
    try {
        get $col
    } catch {
        let full_cellpath = if $opts.items {
            $col
            | split cell-path
            | prepend {value: items, optional: false}
            | into cell-path
        } else {
            $col
        }
        error make {
            msg: $"Cannot find column '($full_cellpath)'",
            label: {
                text: "value originates here",
                span: $opts.span
            },
        }
    }
}

def "error not-a-table" [span: record<start: int, end:int>] {
    error make {
        msg: "input must be a table",
        label: {
            text: "from here",
            span: $span
        },
        help: "Are you using `group-by`? Make sure to use its `--to-table` flag."
    }
}

# Run aggregate operations on output of `group-by --to-table`.
#
# # Example
#
# - group files by type and extension, and get stats about their sizes
# ```nushell
# >_ ls | group-by type { get name | path parse | get extension } --to-table | aggregate size
# ```
# 
# - group data by multiple columns, and run custom aggregate operations
# ```nushell
# >_ open movies.csv
#    | group-by Lead_Studio Genre --to-table
#    | aggregate Worldwide_Gross Profitability --ops {avg: {math avg}, std: {math stddev}}
# ```
#
# - run aggregate operations without grouping the input
# ```nushell
# >_ open movies.csv | aggregate Year
# ```
export def main [
    --ops: record, # default = {min: {math min}, avg: {math avg}, max: {math max}, sum: {math sum}}  
    ...columns: cell-path, # columns to perform aggregations on
]: [
    table -> table<count: int>,
    record -> error,
] {
    let IN = $in
    let md = metadata $in

    let first = try { $IN | first } catch { error not-a-table $md.span }
    if not (($first | describe) starts-with record) {
        error not-a-table $md.span
    }

    let grouped = "items" in $first

    let IN = if $grouped {
        $IN
    } else {
        [{items: $IN}]
    }

    let agg_ops = $ops | default (aggregate-default-ops)

    let results = $IN
    | update items {|group|
        let column_results = $columns
        | each {|col| # col: cell-path
            let column = $group.items | get-item-with-error $col {span: $md.span, items: $grouped}
            let agg_results = $agg_ops | items {|op_name, op| # op_name: string, op: closure
                $column | do $op | wrap (aggregate-col-name $col $op_name)
            }

            for r in $agg_results {
                if ($r | describe) == error {
                    return $r
                }
            }

            $agg_results
            | reduce {|it| merge $it}
        }

        # Manually propagate errors
        for r in $column_results {
            if ($r | describe) == error {
                return $r
            }
        }

        $column_results
        | reduce --fold {} {|it| merge $it}
        | insert count ($group.items | length)
        | roll right  # put count as the first column
    }

    # Manually propagate errors
    for r in $results {
        if ($r.items | describe) == error {
            return $r.items
        }
    }

    $results | flatten items
}
