    
# alias     
    
Define a shortcut for another command.    
    
Usage:    
  > alias <name> <args> <block> {flags}     
    
Parameters:    
  <name> the name of the alias    
  <args> the arguments to the alias    
  <block> the block to run as the body of the alias    
    
Flags:    
  -h, --help: Display this help message    
  -s, --save: save the alias to your config    
    
Examples:    
  An alias without parameters    
  > alias say-hi [] { echo 'Hello!' }    
    
  An alias with a single parameter    
  > alias l [x] { ls $x }    
    
    
# ansi     
    
Output ANSI codes to change color    
    
Usage:    
  > ansi <color> {flags}     
    
Parameters:    
  <color> the name of the color to use or 'reset' to reset the color    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Change color to green    
  > ansi green    
    
  Reset the color    
  > ansi reset    
    
    
# append     
    
Append the given row to the table    
    
Usage:    
  > append <row value> {flags}     
    
Parameters:    
  <row value> the value of the row to append to the table    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Add something to the end of a list or table    
  > echo [1 2 3] | append 4    
    
    
# autoenv     
    
Manage directory specific environments    
    
Usage:    
  > autoenv <subcommand> {flags}     
    
Subcommands:    
  autoenv trust - Trust a .nu-env file in the current or given directory    
  autoenv untrust - Untrust a .nu-env file in the current or given directory    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Allow .nu-env file in current directory    
  > autoenv trust    
    
    
# autoview     
    
View the contents of the pipeline as a table or list.    
    
Usage:    
  > autoview {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Automatically view the results    
  > ls | autoview    
    
  Autoview is also implied. The above can be written as    
  > ls    
    
    
# binaryview     
    
Autoview of binary data.    
    
Usage:    
  > binaryview {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -l, --lores: use low resolution output mode    
    
    
# build-string     
    
Builds a string from the arguments    
    
Usage:    
  > build-string  ...args{flags}     
    
Parameters:    
  ...args: all values to form into the string    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Builds a string from a string and a number, without spaces between them    
  > build-string 'foo' 3    
    
    
# cal     
    
Display a calendar.    
    
Usage:    
  > cal {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -y, --year: Display the year column    
  -q, --quarter: Display the quarter column    
  -m, --month: Display the month column    
  --full-year <integer>: Display a year-long calendar for the specified year    
  --week-start <string>: Display the calendar with the specified day as the first day of the week    
  --month-names: Display the month names instead of integers    
    
Examples:    
  This month's calendar    
  > cal    
    
  The calendar for all of 2012    
  > cal --full-year 2012    
    
  This month's calendar with the week starting on monday    
  > cal --week-start monday    
    
    
# calc     
    
Parse a math expression into a number    
    
Usage:    
  > calc {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Calculate math in the pipeline    
  > echo '10 / 4' | calc    
    
    
# cd     
    
Change to a new path.    
    
Usage:    
  > cd (directory) {flags}     
    
Parameters:    
  (directory) the directory to change to    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Change to a new directory called 'dirname'    
  > cd dirname    
    
  Change to your home directory    
  > cd    
    
  Change to your home directory (alternate version)    
  > cd ~    
    
  Change to the previous directory    
  > cd -    
    
    
# char     
    
Output special characters (eg. 'newline')    
    
Usage:    
  > ansi <character> {flags}     
    
Parameters:    
  <character> the name of the character to output    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Output newline    
  > char newline    
    
    
# clear     
    
clears the terminal    
    
Usage:    
  > clear {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Clear the screen    
  > clear    
    
    
# compact     
    
Creates a table with non-empty rows    
    
Usage:    
  > compact  ...args{flags}     
    
Parameters:    
  ...args: the columns to compact from the table    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Filter out all null entries in a list    
  > echo [1 2 $null 3 $null $null] | compact    
    
  Filter out all directory entries having no 'target'    
  > ls -af | compact target    
    
    
# config     
    
Configuration management.    
    
Usage:    
  > config {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -l, --load <file path>: load the config from the path given    
  -s, --set <any>: set a value in the config, eg) --set [key value]    
  -i, --set_into <string>: sets a variable from values in the pipeline    
  -g, --get <any>: get a value from the config    
  -r, --remove <any>: remove a value from the config    
  -c, --clear: clear the config    
  -p, --path: return the path to the config file    
    
Examples:    
  See all config values    
  > config    
    
  Set completion_mode to circular    
  > config --set [completion_mode circular]    
    
  Store the contents of the pipeline as a path    
  > echo ['/usr/bin' '/bin'] | config --set_into path    
    
  Get the current startup commands    
  > config --get startup    
    
  Remove the startup commands    
  > config --remove startup    
    
  Clear the config (be careful!)    
  > config --clear    
    
  Get the path to the current config file    
  > config --path    
    
    
# count     
    
Show the total number of rows or items.    
    
Usage:    
  > count {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Count the number of entries in a list    
  > echo [1 2 3 4 5] | count    
    
    
# cp     
    
Copy files.    
    
Usage:    
  > cp <src> <dst> {flags}     
    
Parameters:    
  <src> the place to copy from    
  <dst> the place to copy to    
    
Flags:    
  -h, --help: Display this help message    
  -r, --recursive: copy recursively through subdirectories    
    
Examples:    
  Copy myfile to dir_b    
  > cp myfile dir_b    
    
  Recursively copy dir_a to dir_b    
  > cp -r dir_a dir_b    
    
    
# date     
    
Get the current datetime.    
    
Usage:    
  > date {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -u, --utc: use universal time (UTC)    
  -l, --local: use the local time    
  -f, --format <string>: report datetime in supplied strftime format    
  -r, --raw: print date without tables    
    
Examples:    
  Get the current local time and date    
  > date    
    
  Get the current UTC time and date    
  > date --utc    
    
  Get the current time and date and report it based on format    
  > date --format '%Y-%m-%d %H:%M:%S.%f %z'    
    
  Get the current time and date and report it without a table    
  > date --format '%Y-%m-%d %H:%M:%S.%f %z' --raw    
    
    
# debug     
    
Print the Rust debug representation of the values    
    
Usage:    
  > debug {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -r, --raw: Prints the raw value representation.    
    
    
# default     
    
Sets a default row's column if missing.    
    
Usage:    
  > default <column name> <column value> {flags}     
    
Parameters:    
  <column name> the name of the column    
  <column value> the value of the column to default    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Give a default 'target' to all file entries    
  > ls -af | default target 'nothing'    
    
    
# describe     
    
Describes the objects in the stream.    
    
Usage:    
  > describe {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# do     
    
Runs a block, optionally ignoring errors    
    
Usage:    
  > with-env <block> {flags}     
    
Parameters:    
  <block> the block to run     
    
Flags:    
  -h, --help: Display this help message    
  -i, --ignore_errors: ignore errors as the block runs    
    
Examples:    
  Run the block    
  > do { echo hello }    
    
  Run the block and ignore errors    
  > do -i { thisisnotarealcommand }    
    
    
# drop     
    
Drop the last number of rows.    
    
Usage:    
  > drop (rows) {flags}     
    
Parameters:    
  (rows) starting from the back, the number of rows to drop    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Remove the last item of a list/table    
  > echo [1 2 3] | drop    
    
  Remove the last 2 items of a list/table    
  > echo [1 2 3] | drop 2    
    
    
# du     
    
Find disk usage sizes of specified items    
    
Usage:    
  > du (path) {flags}     
    
Parameters:    
  (path) starting directory    
    
Flags:    
  -h, --help: Display this help message    
  -a, --all: Output file sizes as well as directory sizes    
  -r, --deref: Dereference symlinks to their targets for size    
  -x, --exclude <pattern>: Exclude these file names    
  -d, --max-depth <integer>: Directory recursion limit    
  -m, --min-size <integer>: Exclude files below this size    
    
Examples:    
  Disk usage of the current directory    
  > du    
    
    
# each     
    
Run a block on each row of the table.    
    
Usage:    
  > each <block> {flags}     
    
Parameters:    
  <block> the block to run on each row    
    
Flags:    
  -h, --help: Display this help message    
  -n, --numbered: returned a numbered item ($it.index and $it.item)    
    
Examples:    
  Echo the sum of each row    
  > echo [[1 2] [3 4]] | each { echo $it | math sum }    
    
  Echo the square of each integer    
  > echo [1 2 3] | each { echo $(= $it * $it) }    
    
  Number each item and echo a message    
  > echo ['bob' 'fred'] | each --numbered { echo `{{$it.index}} is {{$it.item}}` }    
    
    
# echo     
    
Echo the arguments back to the user.    
    
Usage:    
  > echo  ...args{flags}     
    
Parameters:    
  ...args: the values to echo    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Put a hello message in the pipeline    
  > echo 'hello'    
    
  Print the value of the special '$nu' variable    
  > echo $nu    
    
    
# empty?     
    
Checks emptiness. The last value is the replacement value for any empty column(s) given to check against the table.    
    
Usage:    
  > empty?  ...args{flags}     
    
Parameters:    
  ...args: the names of the columns to check emptiness followed by the replacement value.    
    
Flags:    
  -h, --help: Display this help message    
    
    
# enter     
    
Create a new shell and begin at this path.    
            
Multiple encodings are supported for reading text files by using    
the '--encoding <encoding>' parameter. Here is an example of a few:    
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5    
    
For a more complete list of encodings please refer to the encoding_rs    
documentation link at https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics    
    
Usage:    
  > enter <location> {flags}     
    
Parameters:    
  <location> the location to create a new shell from    
    
Flags:    
  -h, --help: Display this help message    
  -e, --encoding <string>: encoding to use to open file    
    
Examples:    
  Enter a path as a new shell    
  > enter ../projectB    
    
  Enter a file as a new shell    
  > enter package.json    
    
  Enters file with iso-8859-1 encoding    
  > enter file.csv --encoding iso-8859-1    
    
    
# every     
    
Show (or skip) every n-th row, starting from the first one.    
    
Usage:    
  > every <stride> {flags}     
    
Parameters:    
  <stride> how many rows to skip between (and including) each row returned    
    
Flags:    
  -h, --help: Display this help message    
  -s, --skip: skip the rows that would be returned, instead of selecting them    
    
Examples:    
  Get every second row    
  > echo [1 2 3 4 5] | every 2    
    
  Skip every second row    
  > echo [1 2 3 4 5] | every 2 --skip    
    
    
# exit     
    
Exit the current shell (or all shells)    
    
Usage:    
  > exit {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -n, --now: exit out of the shell immediately    
    
Examples:    
  Exit the current shell    
  > exit    
    
  Exit all shells (exiting Nu)    
  > exit --now    
    
    
# fetch     
    
Load from a URL into a cell, convert to table if possible (avoid by appending '--raw')    
    
Usage:    
  > fetch <URL> {flags}     
    
Parameters:    
  <URL> the URL to fetch the contents from    
    
Flags:    
  -h, --help: Display this help message    
  -u, --user <any>: the username when authenticating    
  -p, --password <any>: the password when authenticating    
  -r, --raw: fetch contents as text rather than a table    
    
    
# first     
    
Show only the first number of rows.    
    
Usage:    
  > first (rows) {flags}     
    
Parameters:    
  (rows) starting from the front, the number of rows to return    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Return the first item of a list/table    
  > echo [1 2 3] | first    
    
  Return the first 2 items of a list/table    
  > echo [1 2 3] | first 2    
    
    
# format     
    
Format columns into a string using a simple pattern.    
    
Usage:    
  > format <pattern> {flags}     
    
Parameters:    
  <pattern> the pattern to output. Eg) "{foo}: {bar}"    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Print filenames with their sizes    
  > ls | format '{name}: {size}'    
    
    
# from     
    
Parse content (string or binary) as a table (input format based on subcommand, like csv, ini, json, toml)    
    
Usage:    
  > from <subcommand> {flags}     
    
Subcommands:    
  from csv - Parse text as .csv and create table.    
  from eml - Parse text as .eml and create table.    
  from tsv - Parse text as .tsv and create table.    
  from ssv - Parse text as space-separated values and create a table. The default minimum number of spaces counted as a separator is 2.    
  from ini - Parse text as .ini and create table    
  from bson - Parse binary as .bson and create table.    
  from json - Parse text as .json and create table.    
  from ods - Parse OpenDocument Spreadsheet(.ods) data and create table.    
  from db - Parse binary data as db and create table.    
  from sqlite - Parse binary data as sqlite .db and create table.    
  from toml - Parse text as .toml and create table.    
  from url - Parse url-encoded string as a table.    
  from xlsx - Parse binary Excel(.xlsx) data and create table.    
  from xml - Parse text as .xml and create table.    
  from yaml - Parse text as .yaml/.yml and create table.    
  from yml - Parse text as .yaml/.yml and create table.    
  from ics - Parse text as .ics and create table.    
  from vcf - Parse text as .vcf and create table.    
    
Flags:    
  -h, --help: Display this help message    
    
    
# get     
    
Open given cells as text.    
    
Usage:    
  > get  ...args{flags}     
    
Parameters:    
  ...args: optionally return additional data by path    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Extract the name of files as a list    
  > ls | get name    
    
  Extract the cpu list from the sys information    
  > sys | get cpu    
    
    
# group-by     
    
Creates a new table with the data from the table rows grouped by the column given.    
    
Usage:    
  > group-by (column_name) <subcommand> {flags}     
    
Subcommands:    
  group-by date - Creates a new table with the data from the table rows grouped by the column given.    
    
Parameters:    
  (column_name) the name of the column to group by    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Group items by type    
  > ls | group-by type    
    
  Group items by their value    
  > echo [1 3 1 3 2 1 1] | group-by    
    
    
# headers     
    
Use the first row of the table as column names    
    
Usage:    
  > headers {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Create headers for a raw string    
  > echo "a b c|1 2 3" | split row "|" | split column " " | headers    
    
    
# help     
    
Display help information about commands.    
    
Usage:    
  > help  ...args{flags}     
    
Parameters:    
  ...args: the name of command to get help on    
    
Flags:    
  -h, --help: Display this help message    
    
    
# histogram     
    
Creates a new table with a histogram based on the column name passed in.    
    
Usage:    
  > histogram <column_name>  ...args{flags}     
    
Parameters:    
  <column_name> the name of the column to graph by    
  ...args: column name to give the histogram's frequency column    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Get a histogram for the types of files    
  > ls | histogram type    
    
  Get a histogram for the types of files, with frequency column named count    
  > ls | histogram type count    
    
  Get a histogram for a list of numbers    
  > echo [1 2 3 1 1 1 2 2 1 1] | histogram    
    
    
# history     
    
Display command history.    
    
Usage:    
  > history {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# if     
    
Filter table to match the condition.    
    
Usage:    
  > if <condition> <then_case> <else_case> {flags}     
    
Parameters:    
  <condition> the condition that must match    
  <then_case> block to run if condition is true    
  <else_case> block to run if condition is false    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Run a block if a condition is true    
  > echo 10 | if $it > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }    
    
  Run a block if a condition is false    
  > echo 1 | if $it > 5 { echo 'greater than 5' } { echo 'less than or equal to 5' }    
    
    
# inc     
    
Increment a value or version. Optionally use the column of a table.    
    
Usage:    
  > inc  ...args{flags}     
    
Parameters:    
  ...args: the column(s) to update    
    
Flags:    
  -h, --help: Display this help message    
  -M, --major: increment the major version (eg 1.2.1 -> 2.0.0)    
  -m, --minor: increment the minor version (eg 1.2.1 -> 1.3.0)    
  -p, --patch: increment the patch version (eg 1.2.1 -> 1.2.2)    
    
    
# insert     
    
Insert a new column with a given value.    
    
Usage:    
  > insert <column> <value> {flags}     
    
Parameters:    
  <column> the column name to insert    
  <value> the value to give the cell(s)    
    
Flags:    
  -h, --help: Display this help message    
    
    
# keep     
    
Keep the number of rows only    
    
Usage:    
  > keep (rows) {flags}     
    
Parameters:    
  (rows) starting from the front, the number of rows to keep    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Keep the first row    
  > echo [1 2 3] | keep    
    
  Keep the first four rows    
  > echo [1 2 3 4 5] | keep 4    
    
    
# keep-until     
    
Keeps rows until the condition matches.    
    
Usage:    
  > keep-until <condition> {flags}     
    
Parameters:    
  <condition> the condition that must be met to stop keeping rows    
    
Flags:    
  -h, --help: Display this help message    
    
    
# keep-while     
    
Keeps rows while the condition matches.    
    
Usage:    
  > keep-while <condition> {flags}     
    
Parameters:    
  <condition> the condition that must be met to keep rows    
    
Flags:    
  -h, --help: Display this help message    
    
    
# kill     
    
Kill a process using the process id.    
    
Usage:    
  > kill <pid>  ...args{flags}     
    
Parameters:    
  <pid> process id of process that is to be killed    
  ...args: rest of processes to kill    
    
Flags:    
  -h, --help: Display this help message    
  -f, --force: forcefully kill the process    
  -q, --quiet: won't print anything to the console    
    
Examples:    
  Kill the pid using the most memory    
  > ps | sort-by mem | last | kill $it.pid    
    
  Force kill a given pid    
  > kill --force 12345    
    
    
# last     
    
Show only the last number of rows.    
    
Usage:    
  > last (rows) {flags}     
    
Parameters:    
  (rows) starting from the back, the number of rows to return    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Get the last row    
  > echo [1 2 3] | last    
    
  Get the last three rows    
  > echo [1 2 3 4 5] | last 3    
    
    
# lines     
    
Split single string into rows, one per line.    
    
Usage:    
  > lines {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Split multi-line string into lines    
  > ^echo "two    
lines" | lines    
    
    
# ls     
    
View the contents of the current or given path.    
    
Usage:    
  > ls (path) {flags}     
    
Parameters:    
  (path) a path to get the directory contents from    
    
Flags:    
  -h, --help: Display this help message    
  -a, --all: also show hidden files    
  -f, --full: list all available columns for each entry    
  -s, --short-names: only print the file names and not the path    
  -w, --with-symlink-targets: display the paths to the target files that symlinks point to    
  -d, --du: display the apparent directory size in place of the directory metadata size    
    
Examples:    
  List all files in the current directory    
  > ls    
    
  List all files in a subdirectory    
  > ls subdir    
    
  List all rust files    
  > ls *.rs    
    
    
# match     
    
filter rows by regex    
    
Usage:    
  > match <member> <regex> {flags}     
    
Parameters:    
  <member> the column name to match    
  <regex> the regex to match with    
    
Flags:    
  -h, --help: Display this help message    
    
    
# math     
    
Use mathematical functions as aggregate functions on a list of numbers or tables    
    
Usage:    
  > math <subcommand> {flags}     
    
Subcommands:    
  math avg - Finds the average of a list of numbers or tables    
  math median - Gets the median of a list of numbers    
  math min - Finds the minimum within a list of numbers or tables    
  math mode - Gets the most frequent element(s) from a list of numbers or tables    
  math max - Finds the maximum within a list of numbers or tables    
  math sum - Finds the sum of a list of numbers or tables    
    
Flags:    
  -h, --help: Display this help message    
    
    
# merge     
    
Merge a table.    
    
Usage:    
  > merge <block> {flags}     
    
Parameters:    
  <block> the block to run and merge into the table    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Merge a 1-based index column with some ls output    
  > ls | select name | keep 3 | merge { echo [1 2 3] | wrap index }    
    
    
# mkdir     
    
Make directories, creates intermediary directories as required.    
    
Usage:    
  > mkdir  ...args{flags}     
    
Parameters:    
  ...args: the name(s) of the path(s) to create    
    
Flags:    
  -h, --help: Display this help message    
  -s, --show-created-paths: show the path(s) created.    
    
Examples:    
  Make a directory named foo    
  > mkdir foo    
    
    
# move     
    
moves across desired subcommand.    
    
Usage:    
  > move <subcommand> {flags}     
    
Subcommands:    
  move column - Move columns.    
    
Flags:    
  -h, --help: Display this help message    
    
    
# mv     
    
Move files or directories.    
    
Usage:    
  > mv <source> <destination> {flags}     
    
Parameters:    
  <source> the location to move files/directories from    
  <destination> the location to move files/directories to    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Rename a file    
  > mv before.txt after.txt    
    
  Move a file into a directory    
  > mv test.txt my/subdirectory    
    
  Move many files into a directory    
  > mv *.txt my/subdirectory    
    
    
# n     
    
Go to next shell.    
    
Usage:    
  > n {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# nth     
    
Return only the selected rows    
    
Usage:    
  > nth <row number>  ...args{flags}     
    
Parameters:    
  <row number> the number of the row to return    
  ...args: Optionally return more rows    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Get the second row    
  > echo [first second third] | nth 1    
    
  Get the first and third rows    
  > echo [first second third] | nth 0 2    
    
    
# open     
    
Load a file into a cell, convert to table if possible (avoid by appending '--raw').    
            
Multiple encodings are supported for reading text files by using    
the '--encoding <encoding>' parameter. Here is an example of a few:    
big5, euc-jp, euc-kr, gbk, iso-8859-1, utf-16, cp1252, latin5    
    
For a more complete list of encodings please refer to the encoding_rs    
documentation link at https://docs.rs/encoding_rs/0.8.23/encoding_rs/#statics    
    
Usage:    
  > open <path> {flags}     
    
Parameters:    
  <path> the file path to load values from    
    
Flags:    
  -h, --help: Display this help message    
  -r, --raw: load content as a string instead of a table    
  -e, --encoding <string>: encoding to use to open file    
    
Examples:    
  Opens "users.csv" and creates a table from the data    
  > open users.csv    
    
  Opens file with iso-8859-1 encoding    
  > open file.csv --encoding iso-8859-1 | from csv    
    
    
# p     
    
Go to previous shell.    
    
Usage:    
  > p {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# parse     
    
Parse columns from string data using a simple pattern.    
    
Usage:    
  > parse <pattern> {flags}     
    
Parameters:    
  <pattern> the pattern to match. Eg) "{foo}: {bar}"    
    
Flags:    
  -h, --help: Display this help message    
  -r, --regex: use full regex syntax for patterns    
    
    
# pivot     
    
Pivots the table contents so rows become columns and columns become rows.    
    
Usage:    
  > pivot  ...args{flags}     
    
Parameters:    
  ...args: the names to give columns once pivoted    
    
Flags:    
  -h, --help: Display this help message    
  -r, --header-row: treat the first row as column names    
  -i, --ignore-titles: don't pivot the column names into values    
    
    
# post     
    
Post content to a url and retrieve data as a table if possible.    
    
Usage:    
  > post <path> <body> {flags}     
    
Parameters:    
  <path> the URL to post to    
  <body> the contents of the post body    
    
Flags:    
  -h, --help: Display this help message    
  -u, --user <any>: the username when authenticating    
  -p, --password <any>: the password when authenticating    
  -t, --content-type <any>: the MIME type of content to post    
  -l, --content-length <any>: the length of the content being posted    
  -r, --raw: return values as a string instead of a table    
    
    
# prepend     
    
Prepend the given row to the front of the table    
    
Usage:    
  > prepend <row value> {flags}     
    
Parameters:    
  <row value> the value of the row to prepend to the table    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Add something to the beginning of a list or table    
  > echo [2 3 4] | prepend 1    
    
    
# ps     
    
View information about system processes.    
    
Usage:    
  > ps {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -f, --full: list all available columns for each entry    
    
    
# pwd     
    
Output the current working directory.    
    
Usage:    
  > pwd {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Print the current working directory    
  > pwd    
    
    
# random     
    
Generate random values    
    
Usage:    
  > random <subcommand> {flags}     
    
Subcommands:    
  random bool - Generate a random boolean value    
  random dice - Generate a random dice roll    
  random uuid - Generate a random uuid4 string    
    
Flags:    
  -h, --help: Display this help message    
    
    
# range     
    
Return only the selected rows    
    
Usage:    
  > range <rows > {flags}     
    
Parameters:    
  <rows > range of rows to return: Eg) 4..7 (=> from 4 to 7)    
    
Flags:    
  -h, --help: Display this help message    
    
    
# reject     
    
Remove the given columns from the table.    
    
Usage:    
  > reject  ...args{flags}     
    
Parameters:    
  ...args: the names of columns to remove    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Lists the files in a directory without showing the modified column    
  > ls | reject modified    
    
    
# rename     
    
Creates a new table with columns renamed.    
    
Usage:    
  > rename <column_name>  ...args{flags}     
    
Parameters:    
  <column_name> the new name for the first column    
  ...args: the new name for additional columns    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Rename a column    
  > echo "{a: 1, b: 2, c: 3}" | from json | rename my_column    
    
  Rename many columns    
  > echo "{a: 1, b: 2, c: 3}" | from json | rename spam eggs cars    
    
    
# reverse     
    
Reverses the table.    
    
Usage:    
  > reverse {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Sort list of numbers in descending file size    
  > echo [3 1 2 19 0] | reverse    
    
    
# rm     
    
Remove file(s)    
    
Usage:    
  > rm  ...args{flags}     
    
Parameters:    
  ...args: the file path(s) to remove    
    
Flags:    
  -h, --help: Display this help message    
  -t, --trash: use the platform's recycle bin instead of permanently deleting    
  -p, --permanent: don't use recycle bin, delete permanently    
  -r, --recursive: delete subdirectories recursively    
    
Examples:    
  Delete or move a file to the system trash (depending on 'rm_always_trash' config option)    
  > rm file.txt    
    
  Move a file to the system trash    
  > rm --trash file.txt    
    
  Delete a file permanently    
  > rm --permanent file.txt    
    
    
# run_external     
    
    
    
Usage:    
  > run_external  ...args{flags}     
    
Parameters:    
  ...args: external command arguments    
    
Flags:    
  -h, --help: Display this help message    
    
    
# save     
    
Save the contents of the pipeline to a file.    
    
Usage:    
  > save (path) {flags}     
    
Parameters:    
  (path) the path to save contents to    
    
Flags:    
  -h, --help: Display this help message    
  -r, --raw: treat values as-is rather than auto-converting based on file extension    
    
    
# select     
    
Down-select table to only these columns.    
    
Usage:    
  > select  ...args{flags}     
    
Parameters:    
  ...args: the columns to select from the table    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Select just the name column    
  > ls | select name    
    
  Select the name and size columns    
  > ls | select name size    
    
    
# shells     
    
Display the list of current shells.    
    
Usage:    
  > shells {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# shuffle     
    
Shuffle rows randomly.    
    
Usage:    
  > shuffle {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# size     
    
Gather word count statistics on the text.    
    
Usage:    
  > size {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Count the number of words in a string    
  > echo "There are seven words in this sentence" | size    
    
    
# skip     
    
Skip some number of rows.    
    
Usage:    
  > skip (rows) {flags}     
    
Parameters:    
  (rows) how many rows to skip    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Skip the first 5 rows    
  > echo [1 2 3 4 5 6 7] | skip 5    
    
    
# skip-until     
    
Skips rows until the condition matches.    
    
Usage:    
  > skip-until <condition> {flags}     
    
Parameters:    
  <condition> the condition that must be met to stop skipping    
    
Flags:    
  -h, --help: Display this help message    
    
    
# skip-while     
    
Skips rows while the condition matches.    
    
Usage:    
  > skip-while <condition> {flags}     
    
Parameters:    
  <condition> the condition that must be met to continue skipping    
    
Flags:    
  -h, --help: Display this help message    
    
    
# sort-by     
    
Sort by the given columns, in increasing order.    
    
Usage:    
  > sort-by  ...args{flags}     
    
Parameters:    
  ...args: the column(s) to sort by    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Sort list by increasing value    
  > echo [4 2 3 1] | sort-by    
    
  Sort output by increasing file size    
  > ls | sort-by size    
    
  Sort output by type, and then by file size for each type    
  > ls | sort-by type size    
    
    
# split     
    
split contents across desired subcommand (like row, column) via the separator.    
    
Usage:    
  > split <subcommand> {flags}     
    
Subcommands:    
  split column - splits contents across multiple columns via the separator.    
  split row - splits contents over multiple rows via the separator.    
  split chars - splits a string's characters into separate rows    
    
Flags:    
  -h, --help: Display this help message    
    
    
# split-by     
    
Creates a new table with the data from the inner tables split by the column given.    
    
Usage:    
  > split-by (column_name) {flags}     
    
Parameters:    
  (column_name) the name of the column within the nested table to split by    
    
Flags:    
  -h, --help: Display this help message    
    
    
# start     
    
Opens each file/directory/URL using the default application    
    
Usage:    
  > start  ...args{flags}     
    
Parameters:    
  ...args: files/urls/directories to open    
    
Flags:    
  -h, --help: Display this help message    
  -a, --application <string>: Specifies the application used for opening the files/directories/urls    
    
    
# str     
    
Apply string function.    
    
Usage:    
  > str  ...args<subcommand> {flags}     
    
Subcommands:    
  str to-decimal - converts text into decimal    
  str to-int - converts text into integer    
  str downcase - downcases text    
  str upcase - upcases text    
  str capitalize - capitalizes text    
  str find-replace - finds and replaces text    
  str substring - substrings text    
  str set - sets text    
  str to-datetime - converts text into datetime    
  str trim - trims text    
  str collect - collects a list of strings into a string    
  str length - outputs the lengths of the strings in the pipeline    
    
Parameters:    
  ...args: optionally convert by column paths    
    
Flags:    
  -h, --help: Display this help message    
    
    
# sys     
    
View information about the current system.    
    
Usage:    
  > sys {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# table     
    
View the contents of the pipeline as a table.    
    
Usage:    
  > table {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -n, --start_number <number>: row number to start viewing from    
    
    
# tags     
    
Read the tags (metadata) for values.    
    
Usage:    
  > tags {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# textview     
    
Autoview of text data.    
    
Usage:    
  > textview {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# to     
    
Convert table into an output format (based on subcommand, like csv, html, json, yaml).    
    
Usage:    
  > to <subcommand> {flags}     
    
Subcommands:    
  to bson - Convert table into .bson text.    
  to csv - Convert table into .csv text     
  to html - Convert table into simple HTML    
  to json - Converts table data into JSON text.    
  to sqlite - Convert table to sqlite .db binary data    
  to db - Convert table to db data    
  to md - Convert table into simple Markdown    
  to toml - Convert table into .toml text    
  to tsv - Convert table into .tsv text    
  to url - Convert table into url-encoded text    
  to yaml - Convert table into .yaml/.yml text    
    
Flags:    
  -h, --help: Display this help message    
    
    
# touch     
    
creates a file    
    
Usage:    
  > touch <filename> {flags}     
    
Parameters:    
  <filename> the path of the file you want to create    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Creates "fixture.json"    
  > touch fixture.json    
    
    
# tree     
    
View the contents of the pipeline as a tree.    
    
Usage:    
  > tree {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
    
# trim     
    
Trim leading and following whitespace from text data.    
    
Usage:    
  > trim {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Trims surrounding whitespace and outputs "Hello world"    
  > echo "    Hello world" | trim    
    
    
# uniq     
    
Return the unique rows    
    
Usage:    
  > uniq {flags}     
    
Flags:    
  -h, --help: Display this help message    
  -c, --count: Count the unique rows    
    
    
# update     
    
Update an existing column to have a new value.    
    
Usage:    
  > update <field> <replacement value> {flags}     
    
Parameters:    
  <field> the name of the column to update    
  <replacement value> the new value to give the cell(s)    
    
Flags:    
  -h, --help: Display this help message    
    
    
# version     
    
Display Nu version    
    
Usage:    
  > version {flags}     
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Display Nu version    
  > version    
    
    
# where     
    
Filter table to match the condition.    
    
Usage:    
  > where <condition> {flags}     
    
Parameters:    
  <condition> the condition that must match    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  List all files in the current directory with sizes greater than 2kb    
  > ls | where size > 2kb    
    
  List only the files in the current directory    
  > ls | where type == File    
    
  List all files with names that contain "Car"    
  > ls | where name =~ "Car"    
    
  List all files that were modified in the last two months    
  > ls | where modified <= 2M    
    
    
# which     
    
Finds a program file.    
    
Usage:    
  > which <application> {flags}     
    
Parameters:    
  <application> application    
    
Flags:    
  -h, --help: Display this help message    
  -a, --all: list all executables    
    
    
# with-env     
    
Runs a block with an environment set. Eg) with-env [NAME 'foo'] { echo $nu.env.NAME }    
    
Usage:    
  > with-env <variable> <block> {flags}     
    
Parameters:    
  <variable> the environment variable to temporarily set    
  <block> the block to run once the variable is set    
    
Flags:    
  -h, --help: Display this help message    
    
Examples:    
  Set the MYENV environment variable    
  > with-env [MYENV "my env value"] { echo $nu.env.MYENV }    
    
    
