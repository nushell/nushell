def main [] {
    for test_file in (ls $"($env.FILE_PWD)/test_*.nu" -f | get name) {
        let $module_name = (($test_file | path parse).stem | str replace 'test_' '')
        echo $"Run test file for ($module_name)"
        nu $test_file
    }
}
