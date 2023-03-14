def main [] {
    for test_file in (ls ($env.FILE_PWD | path join "test_*.nu") -f | get name) {
        let $module_name = ($test_file | path parse).stem

        echo $"(ansi yellow)INFO Run tests in ($module_name)(ansi reset)"
        let tests = (
            nu -c $'use ($test_file) *; $nu.scope.commands | to nuon'
            | from nuon
            | where module_name == $module_name
            | where ($it.name | str starts-with "test_")
            | get name
        )

        for test_case in $tests {
            echo $"(ansi grey)DEBUG Run test ($module_name)/($test_case)(ansi reset)"
            nu -c $'use ($test_file) ($test_case); ($test_case)'
        }
    }
}
