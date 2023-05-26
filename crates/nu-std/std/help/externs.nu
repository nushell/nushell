use error.nu extern-not-found-error
use common.nu build-help-header

def "nu-complete list-externs" [] {
    $nu.scope.commands | where is_extern | select name usage | rename value description
}

def build-extern-page [extern: record] {
    let usage = (if not ($extern.usage? | is-empty) {[
        $extern.usage
        ""
    ]} else { [] })

    let rest = [
        (build-help-header -n "Extern")
        $" ($extern.name)"
    ]

    [$usage $rest] | flatten | str join "\n"
}

# Show help on nushell externs.
export def main [
    ...extern: string@"nu-complete list-externs"  # the name of extern to get help on
    --find (-f): string  # string to find in extern names
] {
    let externs = (
        $nu.scope.commands
        | where is_extern
        | select name module_name usage
        | sort-by name
        | str trim
    )

    if not ($find | is-empty) {
        $externs | find $find --columns [name usage]
    } else if not ($extern | is-empty) {
        let found_extern = ($externs | where name == ($extern | str join " "))

        if ($found_extern | is-empty) {
            extern-not-found-error (metadata $extern | get span)
        }

        build-extern-page ($found_extern | get 0)
    } else {
        $externs
    }
}
