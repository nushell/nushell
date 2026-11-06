let name = "foo"
[foo] | where $'($name)' =~ $it
