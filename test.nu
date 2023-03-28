# Short help
#
#
# Long help
# Sooo long!
export def xaccess [ path: list ] {
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
        }

        if ($v | is-empty) {
            return []
        }
    }
    $v
}

export def xreplace-old [ path: list replacement: any ] {
    let t = $in

    if ($path | length) == 1 {
        if $t.tag == $path.0 or $path.0 == '*' {
            $replacement
        } else {
            $t
        }
    } else {
        let step = $path.0
        let rest = ($path | skip 1)

        if $t.tag == $path.0 or $path.0 == '*' {
            let new_content = ($t.content | each {|x| $x | xreplace $rest $replacement})
            {tag: $t.tag attributes: $t.attributes content: $new_content}
        } else {
            $t
        }
    }
}

def xreplace-one-tag [ path: list replacement: any ] {
    let t = $in

    let step = $path.0
    let type = ($step | describe)
    let rest = ($path | skip 1)

    # Get a list of elements to be updated and their indices
    let to_update = ($t.content | enumerate | filter {|it|
        let el = $it.item
        let idx = $it.index

        if $type == 'int' {
            $idx == $step
        } else if $type == 'string' {
            $el.tag == $step
        } else {
            let step_span = (metadata $step).span
            error make {msg: 'Incorrect path step type' 
                    label: {text: 'Use a string or int as a step'
                            start: $step_span.start end: $step_span.end}}
        }
    })

    if ($to_update | is-empty) {
        return $t
    }

    let new_values = ($to_update.item | xreplace $rest $replacement)

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

def xreplace-int-step [ step: int rest: list replacement: any ] {
    let t = $in
    $t | enumerate | each {|it|
        let el = $it.item
        let idx = $it.index

        if $idx == $step {
            [ $el ] | xreplace $rest $replacement | get 0
        } else {
            $el
        }
    }
}

export def xreplace [ path: list replacement: any ] {
    let t = $in

    print $t
    if ($path | is-empty) {
        $t | each {|x| $replacement}
    } else {
        let step = $path.0
        let type = ($step | describe)
        let rest = ($path | skip 1)

        if $type == 'string' {
            $t | each {|x| $x | xreplace-one-tag $path $replacement}
        } else if $type == 'int' {
            $t | xreplace-int-step $step $rest $replacement
        }
    }
    
}