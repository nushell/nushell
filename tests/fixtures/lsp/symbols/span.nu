ls | where name != foo
ls | each { $in }
ls | $in.name
