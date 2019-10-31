# shells

Lists all the active nu shells with a number/index, a name and the path. Also marks the current nu shell.

## Examples

```
> shells
---+---+------------+---------------
 # |   | name       | path 
---+---+------------+---------------
 0 |   | filesystem | /usr 
 1 |   | filesystem | /home 
 2 | X | filesystem | /home/username 
---+---+------------+---------------
```

```
/> shells
---+---+-------------------------------------------------+------------------------------------
 # |   | name                                            | path
---+---+-------------------------------------------------+------------------------------------
 0 |   | filesystem                                      | /Users/username/Code/nushell
 1 | X | {/Users/username/Code/nushell/Cargo.toml}       | /
---+---+-------------------------------------------------+------------------------------------
```
