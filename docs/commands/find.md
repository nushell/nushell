---
title: find
layout: command
version: 0.59.0
---

Searches terms in the input or for elements of the input that satisfies the predicate.

## Signature

```> find ...rest --predicate --regex --insensitive --multiline --dotall --invert```

## Parameters

 -  `...rest`: terms to search
 -  `--predicate {block}`: the predicate to satisfy
 -  `--regex {string}`: regex to match with
 -  `--insensitive`: case-insensitive search for regex (?i)
 -  `--multiline`: multi-line mode: ^ and $ match begin/end of line for regex (?m)
 -  `--dotall`: dotall mode: allow a dot . to match newline character \n for regex (?s)
 -  `--invert`: invert the match

## Examples

Search for multiple terms in a command output
```shell
> ls | find toml md sh
```

Search for a term in a string
```shell
> echo Cargo.toml | find toml
```

Search a number or a file size in a list of numbers
```shell
> [1 5 3kb 4 3Mb] | find 5 3kb
```

Search a char in a list of string
```shell
> [moe larry curly] | find l
```

Find odd values
```shell
> [2 4 3 6 5 8] | find --predicate { |it| ($it mod 2) == 1 }
```

Find if a service is not running
```shell
> [[version patch]; [0.1.0 $false] [0.1.1 $true] [0.2.0 $false]] | find -p { |it| $it.patch }
```

Find using regex
```shell
> [abc bde arc abf] | find --regex "ab"
```

Find using regex case insensitive
```shell
> [aBc bde Arc abf] | find --regex "ab" -i
```

Find value in records
```shell
> [[version name]; [0.1.0 nushell] [0.1.1 fish] [0.2.0 zsh]] | find -r "nu"
```
