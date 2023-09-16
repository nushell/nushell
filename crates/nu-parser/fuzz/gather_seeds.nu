# Check if 'seeds' directory exists. If not, create one.
let seeds_exists = "./seeds" | path exists
if $seeds_exists == false { mkdir seeds }

# Gather all "*.nu" files from '../..' and copy them into 'seeds'
ls ../../**/*.nu | get name | each {|f| cp $f ./seeds/}
