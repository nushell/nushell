def gen_keywords [] {
    let cmds = ($nu.scope.commands
                | where is_extern == false
                and is_custom == false
                and category !~ deprecated
                and ($it.command | str contains -n ' ')
                | get command
                | str join '|')

    let var_with_dash_or_under_regex = '(([a-zA-Z]+[\\-_]){1,}[a-zA-Z]+\\s)'
    let preamble = '\\b('
    let postamble = ')\\b'
    $'"match": "($var_with_dash_or_under_regex)|($preamble)($cmds)($postamble)",'
}
$"Generating keywords(char nl)"
gen_keywords
char nl
char nl

def gen_sub_keywords [] {
    let sub_cmds = ($nu.scope.commands
                    | where is_extern == false
                    and is_custom == false
                    and category !~ deprecated
                    and ($it.command | str contains ' ')
                    | get command)

    let preamble = '\\b('
    let postamble = ')\\b'
    let cmds = (for x in $sub_cmds {
        let parts = ($x | split row ' ')
        $'($parts.0)\\s($parts.1)'
    } | str join '|')
    $'"match": "($preamble)($cmds)($postamble)",'
}
$"Generating sub keywords(char nl)"
gen_sub_keywords
char nl

def gen_keywords_alphabetically [] {
    let alphabet = [a b c d e f g h i j k l m n o p q r s t u v w x y z]
    let cmds = ($nu.scope.commands
                | where is_extern == false
                and is_custom == false
                and category !~ deprecated
                and ($it.command | str contains -n ' ')
                | get command)

    let preamble = '\\b('
    let postamble = ')\\b'


    for alpha in $alphabet {
        let letter_cmds = (for cmd in $cmds {
            if ($cmd | str starts-with $alpha) {
                $cmd
            } else {
                $nothing
            }
        } | str join '|')
        if ($letter_cmds | str trim | str length) > 0 {
            $'"match": "($preamble)($letter_cmds)($postamble)",'
        }
    } | str join "\n"
}

"Generating keywords alphabetically\n"
gen_keywords_alphabetically
char nl

def gen_sub_keywords_alphabetically [] {
    let alphabet = [a b c d e f g h i j k l m n o p q r s t u v w x y z]
    let sub_cmds = ($nu.scope.commands
                    | where is_extern == false
                    and is_custom == false
                    and category !~ deprecated
                    and ($it.command | str contains ' ')
                    | get command)

    let preamble = '\\b('
    let postamble = ')\\b'


    for alpha in $alphabet {
        let letter_cmds = (for cmd in $sub_cmds {
            if ($cmd | str starts-with $alpha) {
                let parts = ($cmd | split row ' ')
                $'($parts.0)\\s($parts.1)'
            } else {
                $nothing
            }
        } | str join '|')
        if ($letter_cmds | str trim | str length) > 0 {
            $'"match": "($preamble)($letter_cmds)($postamble)",'
        }
    } | str join "\n"
}

"Generating sub keywords alphabetically\n"
gen_sub_keywords_alphabetically
char nl