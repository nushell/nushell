# Philosophy

> This document contains philosophical notes about nu.

## Bare Words

In Nu, bare words work the same way they do in most shells.

In most shells, bare words serve two purposes:

```bash
$ ls
# ^^ the name of a command
$ cat Cargo.toml
#     ^^^^^^^^^^ a string
```

Nu adopts this shell idiom.

Consequences:

- Bare words cannot also refer to variables. Variable names are prefixed with `$`.
- Bare words, in almost all contexts, cannot be keywords.
- Numbers and operators aren't bare words.

## One Screen

The utility of a command's output drops off extremely rapidly after a full screen of content.

By default, Nu prefers to present output that can fit into a screen rather than more complete output that spans many screens.

For example, this is the rationale for `ls` returning a flat table containing the files in the current directory, rather than presenting a tree of data by default.
