# help

Use `help` for more information on a command.
Use `help commands` to list all availble commands.
Use `help <command name>` to display help about a particular command.

## Examples

```shell
> help
Welcome to Nushell.

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command

You can also learn more at https://book.nushell.sh
```

```shell
> help commands
━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 #  │ name         │ description 
────┼──────────────┼────────────────────────────────────────────────────────────────────────────────────────
  0 │ add          │ Add a new field to the table. 
  1 │ autoview     │ View the contents of the pipeline as a table or list. 
  2 │ cd           │ Change to a new path. 
  3 │ config       │ Configuration management. 
  4 │ cp           │ Copy files. 
  5 │ date         │ Get the current datetime. 
...
 70 │ trim         │ Trim leading and following whitespace from text data. 
 71 │ version      │ Display Nu version 
 72 │ where        │ Filter table to match the condition. 
 73 │ which        │ Finds a program file. 
━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

```shell
> help cd
Change to a new path.

Usage:
  > cd (directory)
```


