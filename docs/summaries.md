
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
  > [1;36malias[0m[37m [0m[32msay-hi[0m[37m [] { [0m[1;36mecho[0m[37m [0m[32m'Hello!'[0m[37m }[0m

  An alias with a single parameter
  > [1;36malias[0m[37m [0m[32ml[0m[37m [[0m[32mx[0m[37m] { [0m[1;36mls[0m[37m [0m[35m$x[0m[37m }[0m


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
  > [1;36mansi[0m[37m [0m[32mgreen[0m

  Reset the color
  > [1;36mansi[0m[37m [0m[32mreset[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mappend[0m[37m [0m[1;35m4[0m


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
  > [1;36mautoenv trust[0m


# autoview 

View the contents of the pipeline as a table or list.

Usage:
  > autoview {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Automatically view the results
  > [1;36mls[0m[37m | [0m[1;36mautoview[0m

  Autoview is also implied. The above can be written as
  > [1;36mls[0m


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
  > [1;36mbuild-string[0m[37m [0m[32m'foo'[0m[37m [0m[1;35m3[0m


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
  > [1;36mcal[0m

  The calendar for all of 2012
  > [1;36mcal[0m[37m [0m[1;34m--full-year[0m[37m [0m[1;35m2012[0m

  This month's calendar with the week starting on monday
  > [1;36mcal[0m[37m [0m[1;34m--week-start[0m[37m [0m[32mmonday[0m


# calc 

Parse a math expression into a number

Usage:
  > calc {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Calculate math in the pipeline
  > [1;36mecho[0m[37m [0m[32m'10 / 4'[0m[37m | [0m[1;36mcalc[0m


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
  > [1;36mcd[0m[37m [0m[36mdirname[0m

  Change to your home directory
  > [1;36mcd[0m

  Change to your home directory (alternate version)
  > [1;36mcd[0m[37m [0m[36m~[0m

  Change to the previous directory
  > [1;36mcd[0m[37m [0m[36m-[0m


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
  > [1;36mchar[0m[37m [0m[32mnewline[0m


# clear 

clears the terminal

Usage:
  > clear {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Clear the screen
  > [1;36mclear[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[35m$null[0m[37m [0m[1;35m3[0m[37m [0m[35m$null[0m[37m [0m[35m$null[0m[37m] | [0m[1;36mcompact[0m

  Filter out all directory entries having no 'target'
  > [1;36mls[0m[37m [0m[1;34m-af[0m[37m | [0m[1;36mcompact[0m[37m [0m[32mtarget[0m


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
  > [1;36mconfig[0m

  Set completion_mode to circular
  > [1;36mconfig[0m[37m [0m[1;34m--set[0m[37m [[0m[32mcompletion_mode[0m[37m [0m[32mcircular[0m[37m][0m

  Store the contents of the pipeline as a path
  > [1;36mecho[0m[37m [[0m[32m'/usr/bin'[0m[37m [0m[32m'/bin'[0m[37m] | [0m[1;36mconfig[0m[37m [0m[1;34m--set_into[0m[37m [0m[32mpath[0m

  Get the current startup commands
  > [1;36mconfig[0m[37m [0m[1;34m--get[0m[37m [0m[32mstartup[0m

  Remove the startup commands
  > [1;36mconfig[0m[37m [0m[1;34m--remove[0m[37m [0m[32mstartup[0m

  Clear the config (be careful!)
  > [1;36mconfig[0m[37m [0m[1;34m--clear[0m

  Get the path to the current config file
  > [1;36mconfig[0m[37m [0m[1;34m--path[0m


# count 

Show the total number of rows or items.

Usage:
  > count {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Count the number of entries in a list
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m] | [0m[1;36mcount[0m


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
  > [1;36mcp[0m[37m [0m[1;36mmyfile[0m[37m [0m[36mdir_b[0m

  Recursively copy dir_a to dir_b
  > [1;36mcp[0m[37m [0m[1;34m-r[0m[37m [0m[1;36mdir_a[0m[37m [0m[36mdir_b[0m


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
  > [1;36mdate[0m

  Get the current UTC time and date
  > [1;36mdate[0m[37m [0m[1;34m--utc[0m

  Get the current time and date and report it based on format
  > [1;36mdate[0m[37m [0m[1;34m--format[0m[37m [0m[32m'%Y-%m-%d %H:%M:%S.%f %z'[0m

  Get the current time and date and report it without a table
  > [1;36mdate[0m[37m [0m[1;34m--format[0m[37m [0m[32m'%Y-%m-%d %H:%M:%S.%f %z'[0m[37m [0m[1;34m--raw[0m


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
  > [1;36mls[0m[37m [0m[1;34m-af[0m[37m | [0m[1;36mdefault[0m[37m [0m[32mtarget[0m[37m [0m[32m'nothing'[0m


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
  > [1;36mdo[0m[37m { [0m[1;36mecho[0m[37m [0m[32mhello[0m[37m }[0m

  Run the block and ignore errors
  > [1;36mdo[0m[37m [0m[1;34m-i[0m[37m { [0m[32mthisisnotarealcommand[0m[37m }[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mdrop[0m

  Remove the last 2 items of a list/table
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mdrop[0m[37m [0m[1;35m2[0m


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
  > [1;36mdu[0m


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
  > [1;36mecho[0m[37m [[[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m] [[0m[1;35m3[0m[37m [0m[1;35m4[0m[37m]] | [0m[1;36meach[0m[37m { [0m[1;36mecho[0m[37m [0m[35m$it[0m[37m | [0m[1;36mmath sum[0m[37m }[0m

  Echo the square of each integer
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36meach[0m[37m { [0m[1;36mecho[0m[37m $(= [0m[35m$it[0m[37m [0m[33m*[0m[37m [0m[35m$it[0m[37m) }[0m

  Number each item and echo a message
  > [1;36mecho[0m[37m [[0m[32m'bob'[0m[37m [0m[32m'fred'[0m[37m] | [0m[1;36meach[0m[37m [0m[1;34m--numbered[0m[37m { [0m[1;36mecho[0m[37m [0m[1;33m`{{[0m[35m$it.[0m[1;33mindex}}[0m[32m is [0m[1;33m{{[0m[35m$it.[0m[1;33mitem}}`[0m[37m }[0m


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
  > [1;36mecho[0m[37m [0m[32m'hello'[0m

  Print the value of the special '$nu' variable
  > [1;36mecho[0m[37m [0m[35m$nu[0m


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
  > [1;36menter[0m[37m [0m[36m../projectB[0m

  Enter a file as a new shell
  > [1;36menter[0m[37m [0m[36mpackage.json[0m

  Enters file with iso-8859-1 encoding
  > [1;36menter[0m[37m [0m[36mfile.csv[0m[37m [0m[1;34m--encoding[0m[37m [0m[32miso-8859-1[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m] | [0m[1;36mevery[0m[37m [0m[1;35m2[0m

  Skip every second row
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m] | [0m[1;36mevery[0m[37m [0m[1;35m2[0m[37m [0m[1;34m--skip[0m


# exit 

Exit the current shell (or all shells)

Usage:
  > exit {flags} 

Flags:
  -h, --help: Display this help message
  -n, --now: exit out of the shell immediately

Examples:
  Exit the current shell
  > [1;36mexit[0m

  Exit all shells (exiting Nu)
  > [1;36mexit[0m[37m [0m[1;34m--now[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mfirst[0m

  Return the first 2 items of a list/table
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mfirst[0m[37m [0m[1;35m2[0m


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
  > [1;36mls[0m[37m | [0m[1;36mformat[0m[37m [0m[32m'{name}: {size}'[0m


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
  > [1;36mls[0m[37m | [0m[1;36mget[0m[37m [0m[36mname[0m

  Extract the cpu list from the sys information
  > [1;36msys[0m[37m | [0m[1;36mget[0m[37m [0m[36mcpu[0m


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
  > [1;36mls[0m[37m | [0m[1;36mgroup-by[0m[37m [0m[32mtype[0m

  Group items by their value
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m3[0m[37m [0m[1;35m1[0m[37m [0m[1;35m3[0m[37m [0m[1;35m2[0m[37m [0m[1;35m1[0m[37m [0m[1;35m1[0m[37m] | [0m[1;36mgroup-by[0m


# headers 

Use the first row of the table as column names

Usage:
  > headers {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Create headers for a raw string
  > [1;36mecho[0m[37m [0m[32m"a b c|1 2 3"[0m[37m | [0m[1;36msplit row[0m[37m [0m[32m"|"[0m[37m | [0m[1;36msplit column[0m[37m [0m[32m" "[0m[37m | [0m[1;36mheaders[0m


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
  > [1;36mls[0m[37m | [0m[1;36mhistogram[0m[37m [0m[32mtype[0m

  Get a histogram for the types of files, with frequency column named count
  > [1;36mls[0m[37m | [0m[1;36mhistogram[0m[37m [0m[32mtype[0m[37m [0m[32mcount[0m

  Get a histogram for a list of numbers
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m1[0m[37m [0m[1;35m1[0m[37m [0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m2[0m[37m [0m[1;35m1[0m[37m [0m[1;35m1[0m[37m] | [0m[1;36mhistogram[0m


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
  > [1;36mecho[0m[37m [0m[1;35m10[0m[37m | [0m[1;36mif[0m[37m [0m[35m$it[0m[37m [0m[33m>[0m[37m [0m[1;35m5[0m[37m { [0m[1;36mecho[0m[37m [0m[32m'greater than 5'[0m[37m } { [0m[1;36mecho[0m[37m [0m[32m'less than or equal to 5'[0m[37m }[0m

  Run a block if a condition is false
  > [1;36mecho[0m[37m [0m[1;35m1[0m[37m | [0m[1;36mif[0m[37m [0m[35m$it[0m[37m [0m[33m>[0m[37m [0m[1;35m5[0m[37m { [0m[1;36mecho[0m[37m [0m[32m'greater than 5'[0m[37m } { [0m[1;36mecho[0m[37m [0m[32m'less than or equal to 5'[0m[37m }[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mkeep[0m

  Keep the first four rows
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m] | [0m[1;36mkeep[0m[37m [0m[1;35m4[0m


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
  > [1;36mps[0m[37m | [0m[1;36msort-by[0m[37m [0m[32mmem[0m[37m | [0m[1;36mlast[0m[37m | [0m[1;36mkill[0m[37m [0m[35m$it.[0m[1;33mpid[0m

  Force kill a given pid
  > [1;36mkill[0m[37m [0m[1;34m--force[0m[37m [0m[1;35m12345[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mlast[0m

  Get the last three rows
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m] | [0m[1;36mlast[0m[37m [0m[1;35m3[0m


# lines 

Split single string into rows, one per line.

Usage:
  > lines {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Split multi-line string into lines
  > [32m^echo[0m[37m [0m[32m"two
lines"[0m[37m | [0m[1;36mlines[0m


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
  > [1;36mls[0m

  List all files in a subdirectory
  > [1;36mls[0m[37m [0m[1;36msubdir[0m

  List all rust files
  > [1;36mls[0m[37m [0m[1;36m*.rs[0m


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
  > [1;36mls[0m[37m | [0m[1;36mselect[0m[37m [0m[36mname[0m[37m | [0m[1;36mkeep[0m[37m [0m[1;35m3[0m[37m | [0m[1;36mmerge[0m[37m { [0m[1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mwrap[0m[37m [0m[32mindex[0m[37m }[0m


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
  > [1;36mmkdir[0m[37m [0m[36mfoo[0m


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
  > [1;36mmv[0m[37m [0m[1;36mbefore.txt[0m[37m [0m[36mafter.txt[0m

  Move a file into a directory
  > [1;36mmv[0m[37m [0m[1;36mtest.txt[0m[37m [0m[36mmy/subdirectory[0m

  Move many files into a directory
  > [1;36mmv[0m[37m [0m[1;36m*.txt[0m[37m [0m[36mmy/subdirectory[0m


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
  > [1;36mecho[0m[37m [[0m[32mfirst[0m[37m [0m[32msecond[0m[37m [0m[32mthird[0m[37m] | [0m[1;36mnth[0m[37m [0m[1;35m1[0m

  Get the first and third rows
  > [1;36mecho[0m[37m [[0m[32mfirst[0m[37m [0m[32msecond[0m[37m [0m[32mthird[0m[37m] | [0m[1;36mnth[0m[37m [0m[1;35m0[0m[37m [0m[1;35m2[0m


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
  > [1;36mopen[0m[37m [0m[36musers.csv[0m

  Opens file with iso-8859-1 encoding
  > [1;36mopen[0m[37m [0m[36mfile.csv[0m[37m [0m[1;34m--encoding[0m[37m [0m[32miso-8859-1[0m[37m | [0m[1;36mfrom csv[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m] | [0m[1;36mprepend[0m[37m [0m[1;35m1[0m


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
  > [1;36mpwd[0m


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
  > [1;36mls[0m[37m | [0m[1;36mreject[0m[37m [0m[32mmodified[0m


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
  > [1;36mecho[0m[37m [0m[32m"{a: 1, b: 2, c: 3}"[0m[37m | [0m[1;36mfrom json[0m[37m | [0m[1;36mrename[0m[37m [0m[32mmy_column[0m

  Rename many columns
  > [1;36mecho[0m[37m [0m[32m"{a: 1, b: 2, c: 3}"[0m[37m | [0m[1;36mfrom json[0m[37m | [0m[1;36mrename[0m[37m [0m[32mspam[0m[37m [0m[32meggs[0m[37m [0m[32mcars[0m


# reverse 

Reverses the table.

Usage:
  > reverse {flags} 

Flags:
  -h, --help: Display this help message

Examples:
  Sort list of numbers in descending file size
  > [1;36mecho[0m[37m [[0m[1;35m3[0m[37m [0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m19[0m[37m [0m[1;35m0[0m[37m] | [0m[1;36mreverse[0m


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
  > [1;36mrm[0m[37m [0m[1;36mfile.txt[0m

  Move a file to the system trash
  > [1;36mrm[0m[37m [0m[1;34m--trash[0m[37m [0m[1;36mfile.txt[0m

  Delete a file permanently
  > [1;36mrm[0m[37m [0m[1;34m--permanent[0m[37m [0m[1;36mfile.txt[0m


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
  > [1;36mls[0m[37m | [0m[1;36mselect[0m[37m [0m[36mname[0m

  Select the name and size columns
  > [1;36mls[0m[37m | [0m[1;36mselect[0m[37m [0m[36mname[0m[37m [0m[36msize[0m


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
  > [1;36mecho[0m[37m [0m[32m"There are seven words in this sentence"[0m[37m | [0m[1;36msize[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m4[0m[37m [0m[1;35m5[0m[37m [0m[1;35m6[0m[37m [0m[1;35m7[0m[37m] | [0m[1;36mskip[0m[37m [0m[1;35m5[0m


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
  > [1;36mecho[0m[37m [[0m[1;35m4[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m [0m[1;35m1[0m[37m] | [0m[1;36msort-by[0m

  Sort output by increasing file size
  > [1;36mls[0m[37m | [0m[1;36msort-by[0m[37m [0m[32msize[0m

  Sort output by type, and then by file size for each type
  > [1;36mls[0m[37m | [0m[1;36msort-by[0m[37m [0m[32mtype[0m[37m [0m[32msize[0m


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
  > [1;36mtouch[0m[37m [0m[36mfixture.json[0m


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
  > [1;36mecho[0m[37m [0m[32m"    Hello world"[0m[37m | [0m[1;36mtrim[0m


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
  > [1;36mversion[0m


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
  > [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33msize[0m[37m [0m[33m>[0m[37m [0m[1;35m2[0m[1;36mkb[0m

  List only the files in the current directory
  > [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mtype[0m[37m [0m[33m==[0m[37m [0m[32mFile[0m

  List all files with names that contain "Car"
  > [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mname[0m[37m [0m[33m=~[0m[37m [0m[32m"Car"[0m

  List all files that were modified in the last two months
  > [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mmodified[0m[37m [0m[33m<=[0m[37m [0m[1;35m2[0m[1;36mM[0m


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
  > [1;36mwith-env[0m[37m [[0m[32mMYENV[0m[37m [0m[32m"my env value"[0m[37m] { [0m[1;36mecho[0m[37m [0m[35m$nu.[0m[1;33menv[0m[35m.[0m[1;33mMYENV[0m[37m }[0m


# wrap 

Wraps the given data in a table.

Usage:
  > wrap (column) {flags} 

Parameters:
  (column) the name of the new column

Flags:
  -h, --help: Display this help message

Examples:
  Wrap a list into a table with the default column name
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mwrap[0m

  Wrap a list into a table with a given column name
  > [1;36mecho[0m[37m [[0m[1;35m1[0m[37m [0m[1;35m2[0m[37m [0m[1;35m3[0m[37m] | [0m[1;36mwrap[0m[37m [0m[32mMyColumn[0m




