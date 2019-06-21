[![Build Status](https://dev.azure.com/nushell/nushell/_apis/build/status/nushell.nushell?branchName=master)](https://dev.azure.com/nushell/nushell/_build/latest?definitionId=2&branchName=master)

# Nu Shell

Like having a shell in a playground.

# Status

This project is currently in its early stages, though it already works well enough for contributors to dogfood it as their daily driver. Its design is subject to change as it matures.

Nu has a list of built-in commands (listed below). If a command is unknown, the command will shell-out and execute it (using cmd on Windows or bash on Linux and MacOS), correctly passing through stdin, stdout and stderr, so things like your daily git workflows and even `vim` will work just fine.

# Philosophy

Nu draws heavy inspiration from projects like PowerShell. Rather than thinking of you filesystem and services as raw streams of text, Nu looks at each input as something with structure. For example, when you list the contents of a directory, what you get back in a list of objects, where each object represents an item in that directory.

## Pipelines

Nu takes this a step further and builds heavily on the idea of _pipelines_. Just as the Unix philosophy, Nu allows commands to output from stdout and read from stdin. Additionally, commands can output structured data (you can think of this as a third kind of stream). Commands that work in the pipeline fit into one of three categories

* Commands that produce a stream (eg, `ls`)
* Commands that filter a stream (eg, `where "file type" == "Directory"`)
* Commands that consumes the output of the pipeline (eg, `autoview`)

Commands are separated by the pipe symbol (`|`) to denote a pipeline flowing left to right.

```
/home/jonathan/Source/nushell(master)> ls | where "file type" == "Directory" | autoview
-----------+-----------+----------+--------+--------------+----------------
 file name | file type | readonly | size   | accessed     | modified
-----------+-----------+----------+--------+--------------+----------------
 target    | Directory |          | 4.1 KB | 19 hours ago | 19 hours ago
 images    | Directory |          | 4.1 KB | 2 weeks ago  | a week ago
 tests     | Directory |          | 4.1 KB | 2 weeks ago  | 18 minutes ago
 docs      | Directory |          | 4.1 KB | a week ago   | a week ago
 .git      | Directory |          | 4.1 KB | 2 weeks ago  | 25 minutes ago
 src       | Directory |          | 4.1 KB | 2 weeks ago  | 25 minutes ago
 .cargo    | Directory |          | 4.1 KB | 2 weeks ago  | 2 weeks ago
-----------+-----------+----------+--------+--------------+----------------
```

Because most of the time you'll want to see the output of a pipeline, `autoview` is assumed. We could have also written the above:

```
/home/jonathan/Source/nushell(master)> ls | where "file type" == "Directory"
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


## Navigation

By default, Nu opens up into your filesystem and the current working directory. One way to think of this is a pair: the current object and the current path in the object. The filesystem is our first object, and the path is the cwd.

| object | path |
| ------ | ---- |
| Filesystem | /home/jonathan/Source/nushell |

Using the `cd` command allows you to change the path from the current path to a new path, just as you might expect. Using `ls` allows you to view the contents of the filesystem at the current path (or at the path of your choosing).

In addition to `cd` and `ls`, we can `enter` an object. Entering an object makes it the current object to navigate (similar to the concept of mounting a filesystem in Unix systems).

```
/home/jonathan/Source/nushell(master)> enter Cargo.toml
object/>
```

As we enter, we create a stack of objects we're navigating:

| object | path |
| ------ | ---- |
| Filesystem | /home/jonathan/Source/nushell |
| object (from Cargo.toml) | / |

Commands `cd` and `ls` now work on the object being navigated.

```
object/> ls
-----------------+------------------+-----------------
 dependencies    | dev-dependencies | package
-----------------+------------------+-----------------
 [object Object] | [object Object]  | [object Object]
-----------------+------------------+-----------------
```

```
object/> cd package/version
object/package/version> ls
-------
 value
-------
 0.1.2
-------
```

The `exit` command will pop the stack and get us back to a previous object we were navigating.

# Goals

Nu adheres closely to a set of goals that make up its design philosophy. As features are added, they are checked against these goals.

* First and foremost, Nu is cross-platform. Commands and techniques should carry between platforms and offer first-class consistent support for Windows, macOS, and Linux.

* Nu ensures direct compatibility with existing platform-specific executables that make up people's workflows.

* Nu's workflow and tools should have the usability in day-to-day experience of using a shell in 2019 (and beyond).

* Nu views data as both structured and unstructured. It is an object shell like PowerShell.

These goals are all critical, project-defining priorities. Priority #1 is "direct compatibility" because any new shell absolutely needs a way to use existing executables in a direct and natural way.

# Commands
## Initial commands
| command | description |
| ------------- | ------------- |
| cd path | Change to a new path |
| ls (path) | View the contents of the current or given path |
| ps | View current processes |
| sysinfo | View information about the current system |
| open {filename or url} | Load a file into a cell, convert to table if possible (avoid by appending '--raw') |
| enter {filename or url} | Enter (mount) the given contents as the current object |
| exit | Leave/pop from the current object (exits if in filesystem object) |

## Filters on tables (structured data)
| command | description |
| ------------- | ------------- |
| pick ...columns | Down-select table to only these columns |
| reject ...columns | Remove the given columns from the table |
| get column-or-column-path | Open given cells as text |
| sort-by ...columns | Sort by the given columns |
| where condition | Filter table to match the condition |
| skip amount | Skip a number of rows |
| first amount | Show only the first number of rows |
| to-array | Collapse rows into a single list |
| to-json | Convert table into .json text |
| to-toml | Convert table into .toml text |
| to-ini | Convert table into .ini text |

## Filters on text (unstructured data)
| command | description |
| ------------- | ------------- |
| from-ini | Parse text as .ini and create table |
| from-json | Parse text as .json and create table |
| from-toml | Parse text as .toml and create table |
| from-xml | Parse text as .xml and create a table |
| from-yaml | Parse text as a .yaml/.yml and create a table |
| split-column sep ...fields | Split row contents across multiple columns via the separator |
| split-row sep | Split row contents over multiple rows via the separator |
| trim | Trim leading and following whitespace from text data |
| {external-command} $it | Run external command with given arguments, replacing $it with each row text |

## Consuming commands
| command | description |
| ------------- | ------------- |
| autoview | View the contents of the pipeline as a table or list |
| clip | Copy the contents of the pipeline to the copy/paste buffer |
| save filename | Save the contents of the pipeline to a file |
| table | View the contents of the pipeline as a table |
| tree | View the contents of the pipeline as a tree |
| vtable | View the contents of the pipeline as a vertical (rotated) table |

# License

The project is made available under the MIT license. See "LICENSE" for more information.

