[![Build Status](https://dev.azure.com/nushell/nushell/_apis/build/status/nushell.nushell?branchName=master)](https://dev.azure.com/nushell/nushell/_build/latest?definitionId=2&branchName=master) [![Discord](https://img.shields.io/discord/601130461678272522.svg?logo=discord)](https://discord.gg/NtAbbGn)
            
# Nu Shell

Like having a shell in a playground.

# Status

This project is currently in its early stages, though it already works well enough for contributors to dogfood it as their daily driver. Its design is subject to change as it matures.

Nu has a list of built-in commands (listed below). If a command is unknown, the command will shell-out and execute it (using cmd on Windows or bash on Linux and MacOS), correctly passing through stdin, stdout and stderr, so things like your daily git workflows and even `vim` will work just fine.

# Philosophy

Nu draws inspiration from projects like PowerShell, functional programming languages, and modern cli tools. Rather than thinking of files and services as raw streams of text, Nu looks at each input as something with structure. For example, when you list the contents of a directory, what you get back in a list of objects, where each object represents an item in that directory. These values can be piped through a series of steps, in a series of commands called a 'pipeline'.

## Pipelines

In Unix, it's common to pipe between commands to split up a sophisticated command over multiple steps. Nu takes this a step further and builds heavily on the idea of _pipelines_. Just as the Unix philosophy, Nu allows commands to output from stdout and read from stdin. Additionally, commands can output structured data (you can think of this as a third kind of stream). Commands that work in the pipeline fit into one of three categories

* Commands that produce a stream (eg, `ls`)
* Commands that filter a stream (eg, `where type == "Directory"`)
* Commands that consumes the output of the pipeline (eg, `autoview`)

Commands are separated by the pipe symbol (`|`) to denote a pipeline flowing left to right.

```
/home/jonathan/Source/nushell(master)> ls | where type == "Directory" | autoview
--------+-----------+----------+--------+--------------+----------------
 name   | type      | readonly | size   | accessed     | modified
--------+-----------+----------+--------+--------------+----------------
 target | Directory |          | 4.1 KB | 19 hours ago | 19 hours ago
 images | Directory |          | 4.1 KB | 2 weeks ago  | a week ago
 tests  | Directory |          | 4.1 KB | 2 weeks ago  | 18 minutes ago
 docs   | Directory |          | 4.1 KB | a week ago   | a week ago
 .git   | Directory |          | 4.1 KB | 2 weeks ago  | 25 minutes ago
 src    | Directory |          | 4.1 KB | 2 weeks ago  | 25 minutes ago
 .cargo | Directory |          | 4.1 KB | 2 weeks ago  | 2 weeks ago
-----------+-----------+----------+--------+--------------+----------------
```

Because most of the time you'll want to see the output of a pipeline, `autoview` is assumed. We could have also written the above:

```
/home/jonathan/Source/nushell(master)> ls | where type == Directory
```

Being able to use the same commands and compose them differently is an important philosophy in Nu. For example, we could use the built-in `ps` command as well to get a list of the running processes, using the same `where` as above.

```text
C:\Code\nushell(master)> ps | where cpu > 0
------------------ +-----+-------+-------+----------
 name              | cmd | cpu   | pid   | status
------------------ +-----+-------+-------+----------
 msedge.exe        |  -  | 0.77  | 26472 | Runnable
 nu.exe            |  -  | 7.83  | 15473 | Runnable
 SearchIndexer.exe |  -  | 82.17 | 23476 | Runnable
 BlueJeans.exe     |  -  | 4.54  | 10000 | Runnable
-------------------+-----+-------+-------+----------
```

## Opening files

Nu can load file and URL contents as raw text or as structured data (if it recognizes the format). For example, you can load a .toml file as structured data and explore it:

```
/home/jonathan/Source/nushell(master)> open Cargo.toml
-----------------+------------------+-----------------
 dependencies    | dev-dependencies | package
-----------------+------------------+-----------------
 [object Object] | [object Object]  | [object Object]
-----------------+------------------+-----------------
```

We can pipeline this into a command that gets the contents of one of the columns:

```
/home/jonathan/Source/nushell(master)> open Cargo.toml | get package
-------------+----------------------------+---------+---------+------+---------
 authors     | description                | edition | license | name | version
-------------+----------------------------+---------+---------+------+---------
 [list List] | A shell for the GitHub era | 2018    | MIT     | nu   | 0.1.2
-------------+----------------------------+---------+---------+------+---------
```

Finally, we can use commands outside of Nu once we have the data we want:

```
/home/jonathan/Source/nushell(master)> open Cargo.toml | get package.version | echo $it
0.1.2
```

Here we use the variable `$it` to refer to the value being piped to the external command.


## Plugins

Nu supports plugins that offer additional functionality to the shell and follow the same object model that built-in commands use. This allows you to extend nu for your needs.

There are a few examples in the `plugins` directory.

Plugins are binaries that are available in your path and follow a "nu_plugin_*" naming convention. These binaries interact with nu via a simple JSON-RPC protocol where the command identifies itself and passes along its configuration, which then makes it available for use. If the plugin is a filter, data streams to it one element at a time, and it can stream data back in return via stdin/stdout. If the plugin is a sink, it is given the full vector of final data and is given free reign over stdin/stdout to use as it pleases.

# Goals

Nu adheres closely to a set of goals that make up its design philosophy. As features are added, they are checked against these goals.

* First and foremost, Nu is cross-platform. Commands and techniques should carry between platforms and offer first-class consistent support for Windows, macOS, and Linux.

* Nu ensures direct compatibility with existing platform-specific executables that make up people's workflows.

* Nu's workflow and tools should have the usability in day-to-day experience of using a shell in 2019 (and beyond).

* Nu views data as both structured and unstructured. It is an object shell like PowerShell.

* Finally, Nu views data functionally. Rather than using mutation, pipelines act as a mean to load, change, and save data without mutable state.

# Commands
## Initial commands
| command | description |
| ------------- | ------------- |
| cd path | Change to a new path |
| cp source path | Copy files |
| ls (path) | View the contents of the current or given path |
| date (--utc) | Get the current datetime |
| ps | View current processes |
| sys | View information about the current system |
| open {filename or url} | Load a file into a cell, convert to table if possible (avoid by appending '--raw') |
| rm   {file or directory} | Remove a file, (for removing directory append '--recursive') |
| exit | Exit the shell |

## Filters on tables (structured data)
| command | description |
| ------------- | ------------- |
| pick ...columns | Down-select table to only these columns |
| reject ...columns | Remove the given columns from the table |
| get column-or-column-path | Open given cells as text |
| sort-by ...columns | Sort by the given columns |
| where condition | Filter table to match the condition |
| inc (field) | Increment a value or version. Optional use the field of a table |
| add field value | Add a new field to the table |
| edit field value | Edit an existing field to have a new value |
| skip amount | Skip a number of rows |
| first amount | Show only the first number of rows |
| to-array | Collapse rows into a single list |
| to-json | Convert table into .json text |
| to-toml | Convert table into .toml text |
| to-yaml | Convert table into .yaml text |
| to-csv  | Convert table into .csv text  |

## Filters on text (unstructured data)
| command | description |
| ------------- | ------------- |
| from-csv | Parse text as .csv and create table |
| from-ini | Parse text as .ini and create table |
| from-json | Parse text as .json and create table |
| from-toml | Parse text as .toml and create table |
| from-xml | Parse text as .xml and create a table |
| from-yaml | Parse text as a .yaml/.yml and create a table |
| lines | Split single string into rows, one per line |
| size | Gather word count statistics on the text |
| split-column sep ...fields | Split row contents across multiple columns via the separator |
| split-row sep | Split row contents over multiple rows via the separator |
| trim | Trim leading and following whitespace from text data |
| {external-command} $it | Run external command with given arguments, replacing $it with each row text |

## Consuming commands
| command | description |
| ------------- | ------------- |
| autoview | View the contents of the pipeline as a table or list |
| binaryview | Autoview of binary data |
| clip | Copy the contents of the pipeline to the copy/paste buffer |
| save filename | Save the contents of the pipeline to a file |
| table | View the contents of the pipeline as a table |
| tree | View the contents of the pipeline as a tree |
| vtable | View the contents of the pipeline as a vertical (rotated) table |

# License

The project is made available under the MIT license. See "LICENSE" for more information.

