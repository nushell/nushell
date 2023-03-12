# Utility functions to read, change and create XML data in format supported
# by `to xml` and `from xml` commands

# Get all xml entries matching simple xpath-inspired query
export def xaccess [
    path: list # List of steps. Each step can be a
               # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
               # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
               # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
] {
    let t = $in
    if ($path | is-empty) {
        let path_span = (metadata $path).span
        error make {msg: 'Empty path provided'
                    label: {text: 'Use a non-empty  list of path steps'
                            start: $path_span.start end: $path_span.end}}
    }
    # In xpath first element in path is applied to root element
    # this way it is possible to apply first step to root element
    # of nu xml without unrolling one step of loop
    mut v = {content: [ { content: $t } ] }
    for $s in ($path) {
        let type = ($s | describe)
        if $type == 'string' {
            if $s == '*' {
                $v = ($v.content | flatten)
            } else {
                $v = ($v.content | flatten | where tag == $s)
            }
        } else if $type == 'int' {
            $v = [ ($v | get $s) ]
        } else {
            let step_span = (metadata $s).span
            error make {msg: 'Incorrect path step type'
                    label: {text: 'Use a string or int as a step'
                            start: $step_span.start end: $step_span.end}}
        }

        if ($v | is-empty) {
            return []
        }
    }
    $v
}

def xupdate-string-step [ step: string rest: list updater: closure ] {
    let t = $in

    # Get a list of elements to be updated and their indices
    let to_update = ($t.content | enumerate | filter {|it|
        let el = $it.item
        $step == '*' or $el.tag == $step
    })

    if ($to_update | is-empty) {
        return $t
    }

    let new_values = ($to_update.item | xupdate-internal $rest $updater)

    mut reenumerated_new_values = ($to_update.index | zip $new_values | each {|x| {index: $x.0 item: $x.1}})

    mut new_content = []
    for it in ($t.content | enumerate) {
        let el = $it.item
        let idx = $it.index

        let next = (if (not ($reenumerated_new_values | is-empty)) and $idx == $reenumerated_new_values.0.index {
            let tmp = $reenumerated_new_values.0
            $reenumerated_new_values = ($reenumerated_new_values | skip 1)
            $tmp.item
        } else {
            $el
        })

        $new_content = ($new_content | append $next)
    }

    {tag: $t.tag attributes: $t.attributes content: $new_content}
}

def xupdate-int-step [ step: int rest: list updater: closure ] {
    let t = $in
    $t | enumerate | each {|it|
        let el = $it.item
        let idx = $it.index

        if $idx == $step {
            [ $el ] | xupdate-internal $rest $updater | get 0
        } else {
            $el
        }
    }
}

def xupdate-internal [ path: list updater: closure ] {
    let t = $in

    if ($path | is-empty) {
        $t | each $updater
    } else {
        let step = $path.0
        let type = ($step | describe)
        let rest = ($path | skip 1)

        if $type == 'string' {
            $t | each {|x| $x | xupdate-string-step $step $rest $updater}
        } else if $type == 'int' {
            $t | xupdate-int-step $step $rest $updater
        } else {
            let step_span = (metadata $step).span
            error make {msg: 'Incorrect path step type'
                    label: {text: 'Use a string or int as a step'
                            start: $step_span.start end: $step_span.end}}
        }
    }

}

# Update xml data entries matching simple xpath-inspired query
export def xupdate [
    path: list  # List of steps. Each step can be a
                # 1. String with tag name. Finds all children with specified name. Equivalent to `child::A` in xpath
                # 2. `*` string. Get all children without any filter. Equivalent to `descendant` in xpath
                # 3. Int. Select n-th among nodes selected by previous path. Equivalent to `(...)[1]` in xpath, but is indexed from 0.
    updater: closure # A closure used to transform entries matching path.
] {
    {tag:? attributes:? content: [$in]} | xupdate-internal $path $updater | get content.0
}