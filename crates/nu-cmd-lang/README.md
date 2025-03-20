# nu-cmd-lang

## the base language and command crate of nu

The commands in this crate are the *core commands* of the nu language.
It is also the base crate upon which all other command crates sit on
top of including:

* nu-command
* nu-cli
* nu-cmd-extra

As time goes on and the nu language develops further in parallel with nushell we will be adding other command crates to the system.

### What does it mean to be a base crate ?

A base crate is one with minimal dependencies in our system so that other developers can come along and use this crate without having a lot of baggage in terms of other crates which will bloat their underlying application.

### Background on nu-cmd-lang

This crate was designed to be a small, concise set of tools or commands that serve as the *foundation layer* of both nu and nushell. These are the core commands needed to have a nice working version of the *nu language* without all of the support that the other commands provide inside nushell. Prior to the launch of this crate all of our commands were housed in the crate *nu-command*. Moving forward we would like to *slowly* break out the commands in nu-command into different crates; the naming and how this will work and where all the commands will be located is a "work in progress" especially now that the *standard library* is starting to become more popular as a location for commands. As time goes on some of our commands written in rust will be migrated to nu and when this happens they will be moved into the *standard library*.
