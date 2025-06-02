# Custom commands to read, change and create XML data in format supported by the `to xml` and `from xml` commands

# Get all xml entries matching simple xpath-inspired query
export def xaccess [
    path: list # List of steps. Each step can be a
               # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
               # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
               # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
               # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true.
] {
    let input = $in
    if ($path | is-empty) {
        let path_span = (metadata $path).span
        error make {
            msg: 'Empty path provided'
            label: {
                text: 'Use a non-empty  list of path steps'
                span: $path_span
            }
        }
    }
    # In xpath first element in path is applied to root element
    # this way it is possible to apply first step to root element
    # of nu xml without unrolling one step of loop
    mut values: any = ()
    $values = {content: [ { content: $input } ] }
    for $step in ($path) {
        match ($step | describe) {
            'string' => {
                if $step == '*' {
                    $values = ($values.content | flatten)
                } else {
                    $values = ($values.content | flatten | where tag == $step)
                }
            },
            'int' => {
                $values = [ ($values | get $step) ]
            },
            'closure' => {
                $values = ($values | where {|x| do $step $x})
            },
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

        if ($values | is-empty) {
            return []
        }
    }
    $values
}

def xupdate-string-step [ step: string rest: list updater: closure ] {
    let input = $in

    # Get a list of elements to be updated and their indices
    let to_update = ($input.content | enumerate | where {|it|
        let item = $it.item
        $step == '*' or $item.tag == $step
    })

    if ($to_update | is-empty) {
        return $input
    }

    let new_values = ($to_update.item | xupdate-internal $rest $updater)

    mut reenumerated_new_values: any = ($to_update.index | zip $new_values | each {|x| {index: $x.0 item: $x.1}})

    mut new_content = []
    for it in ($input.content | enumerate) {
        let item = $it.item
        let idx = $it.index

        let next = (if (not ($reenumerated_new_values | is-empty)) and $idx == $reenumerated_new_values.0.index {
            let tmp = $reenumerated_new_values.0
            $reenumerated_new_values = ($reenumerated_new_values | skip 1)
            $tmp.item
        } else {
            $item
        })

        $new_content = ($new_content | append $next)
    }

    {tag: $input.tag attributes: $input.attributes content: $new_content}
}

def xupdate-int-step [ step: int rest: list updater: closure ] {
    $in | enumerate | each {|it|
        let item = $it.item
        let idx = $it.index

        if $idx == $step {
            [ $item ] | xupdate-internal $rest $updater | get 0
        } else {
            $item
        }
    }
}

def xupdate-closure-step [ step: closure rest: list updater: closure ] {
    $in | each {|it|
        if (do $step $it) {
            [ $it ] | xupdate-internal $rest $updater | get 0
        } else {
            $it
        }
    }
}

def xupdate-internal [ path: list updater: closure ] {
    let input = $in

    if ($path | is-empty) {
        $input | each $updater
    } else {
        let step = $path.0
        let rest = ($path | skip 1)

        match ($step | describe) {
            'string' => {
                $input | each {|x| $x | xupdate-string-step $step $rest $updater}
            },
            'int' => {
                $input | xupdate-int-step $step $rest $updater
            },
            'closure' => {
                $input | xupdate-closure-step $step $rest $updater
            },
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

# Update XML data entries matching simple xpath-inspired query
export def xupdate [
    path: list  # List of steps. Each step can be a
                # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
                # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
                # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
                # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true.
    updater: closure # A closure used to transform entries matching path.
] {
    {tag:? attributes:? content: [$in]} | xupdate-internal $path $updater | get content.0
}

# Get type of an XML entry
#
# Possible types are 'tag', 'text', 'pi' and 'comment'
export def xtype [] {
    let input = $in
    if (($input | describe) == 'string' or
        ($input.tag? == null and $input.attributes? == null and ($input.content? | describe) == 'string')) {
        'text'
    } else if $input.tag? == '!' {
        'comment'
    } else if $input.tag? != null and ($input.tag? | str starts-with '?') {
        'pi'
    } else if $input.tag? != null {
        'tag'
    } else {
        error make {msg: 'Not an xml emtry. Check valid types of xml entries via `help to xml`'}
    }
}

# Insert new entry to elements matching simple xpath-inspired query
export def xinsert [
    path: list  # List of steps. Each step can be a
                # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
                # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
                # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
                # 4. Closure. Predicate accepting entry. Selects all entries among nodes selected by previous path for which predicate returns true.
    new_entry: record # A new entry to insert into `content` field of record at specified position
    position?: int  # Position to insert `new_entry` into. If specified inserts entry at given position (or end if
                    # position is greater than number of elements) in content of all entries of input matched by
                    # path. If not specified inserts at the end.
] {
    $in | xupdate $path {|entry|
        match ($entry | xtype) {
            'tag' => {
                let new_content = if $position == null {
                    $entry.content | append $new_entry
                } else {
                    let position = if $position > ($entry.content | length) {
                        $entry.content | length
                    } else {
                        $position
                    }
                    $entry.content | insert $position $new_entry
                }


                {tag: $entry.tag attributes: $entry.attributes content: $new_content}
            },
            _ => (error make {msg: 'Can insert entry only into content of a tag node'})
        }
    }
}
