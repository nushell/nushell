# Nu Shell

A shell for the GitHub era. Like having a playground for a shell.

# Status

This project has little of what will eventually be necessary for Nu to serve as your day-to-day shell. It already works well enough for contributors to dogfood it as their daily driver, but there are too many basic deficiencies for it to be useful for most people.

At the moment, executing a command that isn't identified as a built-in new command will fall back to running it as a shell command (using cmd on Windows or bash on Linux and MacOS), correctly passing through stdin, stdout and stderr, so things like your daily git workflows and even `vim` will work just fine.

## Commands
| command | description |
| ------------- | ------------- | 
| cd directory | Change to the given directory |
| ls | View current directory contents |
| ps | View current processes |
| open filename | Load a file into a cell, convert to table if possible (avoid by appending '--raw') |

## Commands on tables
| command | description |
| ------------- | ------------- | 
| pick ...columns | Down-select table to only these columns |
| reject ...columns | Remove the given columns from the table |
| select column-or-column-path | Open given cells as text |
| sort-by ...columns | Sort by the given columns |
| where condition | Filter table to match the condition |
| skip amount | Skip a number of rows |
| first amount | Show only the first number of rows |
| to-array | Collapse rows into a single list |
| to-json | Convert table into .json text |
| to-toml | Convert table into .toml text |

## Commands on text
| command | description |
| ------------- | ------------- | 
| from-json | Parse text as .json and create table |
| from-toml | Parse text as .toml and create table |
| split-column sep ...fields | Split row contents across multiple columns via the separator |
| split-row sep | Split row contents over multiple rows via the separator |
| trim | Trim leading and following whitespace from text data |
| {external-command} $it | Run external command with given arguments, replacing $it with each row text | 

# Goals

Prime Directive: Cross platform workflows, with first-class consistent support for Windows, OSX and Linux.

Priority #1: direct compatibility with existing platform-specific executables that make up people's workflows

Priority #2: Create workflow tools that more closely match the day-to-day experience of using a shell in 2019 (and beyond)

Priority #3: It's an object shell like PowerShell.

> These goals are all critical, project-defining priorities. Priority #1 is "direct compatibility" because any new shell absolutely needs a way to use existing executables in a direct and natural way.

# License

The project is made available under the MIT license. See "LICENSE" for more information.

# A Taste of Nu

Nu has built-in commands for ls and ps, loading these results into a table you can work with.

```text
~\Code\nushell> ps | where cpu > 0
------------------------------------------------
 name               cmd  cpu    pid    status
------------------------------------------------
 msedge.exe         -    0.77   26472  Runnable
------------------------------------------------
 nu.exe             -    7.83   15473  Runnable
------------------------------------------------
 SearchIndexer.exe  -    82.17  23476  Runnable
------------------------------------------------
 BlueJeans.exe      -    4.54   10000  Runnable
------------------------------------------------
```

Commands are linked together with pipes, allowing you to select the data you want to use.

```text
~\Code\nushell> ps | where name == chrome.exe | first 5
----------------------------------------
 name        cmd  cpu   pid    status
----------------------------------------
 chrome.exe  -    0.00  22092  Runnable
----------------------------------------
 chrome.exe  -    0.00  17324  Runnable
----------------------------------------
 chrome.exe  -    0.00  16376  Runnable
----------------------------------------
 chrome.exe  -    0.00  21876  Runnable
----------------------------------------
 chrome.exe  -    0.00  13432  Runnable
----------------------------------------
```

The name of the columns in the table can be used to sort the table.

```text
~\Code\nushell> ls | sort-by "file type" size
----------------------------------------------------------------------------------------
 file name      file type  readonly  size      created       accessed      modified
----------------------------------------------------------------------------------------
 .cargo         Directory            Empty     a week ago    a week ago    a week ago
----------------------------------------------------------------------------------------
 .git           Directory            Empty     2 weeks ago   9 hours ago   9 hours ago
----------------------------------------------------------------------------------------
 images         Directory            Empty     2 weeks ago   2 weeks ago   2 weeks ago
----------------------------------------------------------------------------------------
 src            Directory            Empty     2 weeks ago   10 hours ago  10 hours ago
----------------------------------------------------------------------------------------
 target         Directory            Empty     10 hours ago  10 hours ago  10 hours ago
----------------------------------------------------------------------------------------
 tests          Directory            Empty     14 hours ago  10 hours ago  10 hours ago
----------------------------------------------------------------------------------------
 tmp            Directory            Empty     2 days ago    2 days ago    2 days ago
----------------------------------------------------------------------------------------
 rustfmt.toml   File                 16 B      a week ago    a week ago    a week ago
----------------------------------------------------------------------------------------
 .gitignore     File                 32 B      2 weeks ago   2 weeks ago   2 weeks ago
----------------------------------------------------------------------------------------
 .editorconfig  File                 156 B     2 weeks ago   2 weeks ago   2 weeks ago
----------------------------------------------------------------------------------------
```

You can also use the names of the columns to down-select to only the data you want.
```text
~\Code\nushell> ls | pick "file name" "file type" size | sort-by "file type"
------------------------------------
 file name      file type  size
------------------------------------
 .cargo         Directory  Empty
------------------------------------
 .git           Directory  Empty
------------------------------------
 images         Directory  Empty
------------------------------------
 src            Directory  Empty
------------------------------------
 target         Directory  Empty
------------------------------------
 tests          Directory  Empty
------------------------------------
 rustfmt.toml   File       16 B
------------------------------------
 .gitignore     File       32 B
------------------------------------
 .editorconfig  File       156 B
------------------------------------
```

Some file types can be loaded as tables.

```text
~\Code\nushell> open Cargo.toml
----------------------------------------------------
 dependencies     dev-dependencies  package
----------------------------------------------------
 [object Object]  [object Object]   [object Object]
----------------------------------------------------

~\Code\nushell> open Cargo.toml | select package
--------------------------------------------------------------------------
 authors      description                 edition  license  name  version
--------------------------------------------------------------------------
 [list List]  A shell for the GitHub era  2018     MIT      nu    0.1.1
--------------------------------------------------------------------------
```

Once you've found the data, you can call out to external applications and use it.

```text
~\Code\nushell> open Cargo.toml | select package.version | echo $it
0.1.1
```

Nu currently has fish-style completion of previous commands, as well ctrl-r reverse search.

![autocompletion][fish-style]

[fish-style]: ./images/nushell-autocomplete3.gif "Fish-style autocomplete"
