# Nu Shell

A shell for the GitHub era. A shell you can hack on.

# Status

This project has little of what will eventually be necessary for Nu to serve as your day-to-day shell. It already works well enough for contributors to dogfood it as their daily driver, but there are too many basic deficiencies for it to be useful for most people.

At the moment, executing a command that isn't identified as a built-in new command will fall back to running it as a shell command (using cmd on Windows or bash on Linux and MacOS), correctly passing through stdin, stdout and stderr, so things like your daily git workflows and even `vim` will work just fine.

There is not yet support for piping external commands to each other; piping is limited to Nu commands at the moment.

Nu currently has the following built-in commands:

-   cd directory
-   ls
-   ps
-   column ...fields 
-   reject ...fields
-   sort-by ...fields
-   where condition
-   skip amount
-   first amount
-   to-array
-   to-json
-   from-json
-   from-toml
-   open filename
-   split-column sep ...fields
-   split-row sep ...fields
-   select field 
-   trim

# Goals

Prime Directive: Cross platform workflows, with first-class consistent support for Windows, OSX and Linux.

Priority #1: direct compatibility with existing platform-specific executables that make up people's workflows

Priority #2: Create workflow tools that more closely match the day-to-day experience of using a shell in 2019 (and beyond)

Priority #3: It's an object shell like PowerShell.

> These goals are all critical, project-defining priorities. Priority #1 is "direct compatibility" because any new shell absolutely needs a way to use existing executables in a direct and natural way.

# A Taste of Nu

```text
~\Code\nushell> ps | where cpu > 0
+-------------------+-----+-------+-------+----------+
| name              | cmd | cpu   | pid   | status   |
+-------------------+-----+-------+-------+----------+
| chrome.exe        | -   | 7.83  | 10508 | Runnable |
+-------------------+-----+-------+-------+----------+
| SearchIndexer.exe | -   | 7.83  | 4568  | Runnable |
+-------------------+-----+-------+-------+----------+
| nu.exe            | -   | 54.83 | 15436 | Runnable |
+-------------------+-----+-------+-------+----------+
| chrome.exe        | -   | 7.83  | 10000 | Runnable |
+-------------------+-----+-------+-------+----------+
| BlueJeans.exe     | -   | 7.83  | 6968  | Runnable |
+-------------------+-----+-------+-------+----------+

~\Code\nushell> ps | where name == chrome.exe | take 10

+------------+-----+------+-------+----------+
| name       | cmd | cpu  | pid   | status   |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 22092 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 17324 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 16376 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 21876 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 13432 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 11772 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 13796 | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 1608  | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 3340  | Runnable |
+------------+-----+------+-------+----------+
| chrome.exe | -   | 0.00 | 20268 | Runnable |
+------------+-----+------+-------+----------+

~\Code\nushell> ls | sort-by "file type" size
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| file name     | file type | readonly | size     | created        | accessed       | modified       |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| .git          | Directory |          | Empty    | a week ago     | 2 minutes ago  | 2 minutes ago  |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| src           | Directory |          | Empty    | a week ago     | 42 minutes ago | 42 minutes ago |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| target        | Directory |          | Empty    | a day ago      | 19 hours ago   | 19 hours ago   |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| .gitignore    | File      |          | 30 B     | a week ago     | 2 days ago     | 2 days ago     |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| .editorconfig | File      |          | 148 B    | 6 days ago     | 6 days ago     | 6 days ago     |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| Cargo.toml    | File      |          | 714 B    | 42 minutes ago | 42 minutes ago | 42 minutes ago |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| history.txt   | File      |          | 1.4 KiB  | 2 days ago     | 30 minutes ago | 30 minutes ago |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| README.md     | File      |          | 2.3 KiB  | an hour ago    | 30 seconds ago | 30 seconds ago |
+---------------+-----------+----------+----------+----------------+----------------+----------------+
| Cargo.lock    | File      |          | 38.6 KiB | 42 minutes ago | 42 minutes ago | 42 minutes ago |
+---------------+-----------+----------+----------+----------------+----------------+----------------+

~\Code\nushell> ls | column "file name" "file type" size | sort-by "file type"
+---------------+-----------+----------+
| file name     | file type | size     |
+---------------+-----------+----------+
| .git          | Directory | Empty    |
+---------------+-----------+----------+
| src           | Directory | Empty    |
+---------------+-----------+----------+
| target        | Directory | Empty    |
+---------------+-----------+----------+
| .editorconfig | File      | 148 B    |
+---------------+-----------+----------+
| .gitignore    | File      | 30 B     |
+---------------+-----------+----------+
| Cargo.lock    | File      | 38.6 KiB |
+---------------+-----------+----------+
| Cargo.toml    | File      | 714 B    |
+---------------+-----------+----------+
| history.txt   | File      | 1.4 KiB  |
+---------------+-----------+----------+
| README.md     | File      | 2.3 KiB  |
+---------------+-----------+----------+
```

Nu currently has fish-style completion of previous commands, as well ctrl-r reverse search.

![autocompletion][fish-style]

[fish-style]: ./images/nushell-autocomplete.gif "Fish-style autocomplete"
