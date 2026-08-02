export use toolkit *

@search-terms toolkit
@example "List all toolkit subcommands" { toolkit }
export def main [] {
    let cmds = (scope commands | where name =~ '^toolkit ' and name != 'toolkit toolkit' | sort-by name | select name description | each {|r| {name: ($r.name | str replace --regex '^toolkit ' ''), description: $r.description}})
    if ($cmds | is-empty) {
        return
    }
    print $"Nushell Development Toolkit"
    print ""
    print "Usage: toolkit <command>"
    print ""
    $cmds | table
}
