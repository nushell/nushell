# nu-source

## Overview

The `nu-source` crate contains types and traits used for keeping track of _metadata_ about values being processed.
Nu uses `Tag`s to keep track of where a value came from, an `AnchorLocation`,
as well as positional information about the value, a `Span`.
An `AnchorLocation` can be a `Url`, `File`, or `Source` text that a value was parsed from.
The source `Text` is special in that it is a type similar to a `String` that comes with the ability to be cheaply cloned.
A `Span` keeps track of a value's `start` and `end` positions.
These types make up the metadata for a value and are wrapped up together in a `Tagged` struct,
which holds everything needed to track and locate a value.

Nu's metadata system can be seen when reporting errors.
In the following example Nu is able to report to the user where the typo of a column originated from.

```shell
1 | ls | get typ
  |          ^^^ did you mean 'type'?
```

In addition to metadata tracking, `nu-source` also contains types and traits
related to debugging, tracing, and formatting the metadata and values it processes.

## Other Resources

- [Nushell Github Project](https://github.com/nushell):
  Contains all projects in the Nushell ecosystem such as the source code to Nushell as well as website and books.
- [Nushell Git Repository](https://github.com/nushell/nushell):
  A direct link to the source git repository for Nushell
- [Nushell Contributor Book](https://github.com/nushell/contributor-book):
  An overview of topics about Nushell to help you get started contributing to the project.
- [Discord Channel](https://discordapp.com/invite/NtAbbGn)
- [Twitter](https://twitter.com/nu_shell)
