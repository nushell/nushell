def collect-modules [
    path: path,
    module?: string
] {
    let tests_path = ($path | default $env.FILE_PWD)
    let module_search = ($module | default "test_*")
    (ls ($tests_path | path join $"**/($module_search).nu") -f | get name)    
}

def collect-commands [
    test_file: string,
    module_name: string,
    command?: string
] {
    let commands = (
        nu -c $'use ($test_file) *; $nu.scope.commands | to nuon'
        | from nuon
        | where module_name == $module_name
        | where ($it.name | str starts-with "test_")
        | get name
    )
    if $command == null {
        $commands
    } else {
        $commands | where $it == $command
    }
}

# Test executor
#
# It executes exported "test_*" commands in "test_*" modules
def main [
    --path: path, # Path to look for tests. Default: directory of this file.
    --module: string, # Module to run tests. Default: all test modules found.
    --command: string, # Test command to run. Default: all test command found in the files.
    --list, # Do not run any tests, just list them (dry run)
] {
    let dry_run = ($list | default false)
    for test_file in (collect-modules $path $module) {
        let $module_name = ($test_file | path parse).stem

        echo $"(ansi default)INFO  Run tests in ($module_name)(ansi reset)"
        let tests = (collect-commands $test_file $module_name $command)

        for test_case in $tests {
            echo $"(ansi default_dimmed)DEBUG Run test ($module_name)/($test_case)(ansi reset)"
            if $dry_run {
                continue
            }

            nu -c $'use ($test_file) ($test_case); ($test_case)'
        }
    }
}
