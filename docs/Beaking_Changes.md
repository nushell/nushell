# Beaking Changes

This file attempts to list all breaking changes that came with the new engine update.

## Variable Name Changes

* `$nu.home-dir` is now called `$nu.home-path`
* `$nu.temp-dir` is now called `$nu.temp-path`
* All config is now contained within `$config` which can be initialized by `config.nu`. There is no `config.toml` anymore.

## `main` Command in Scripts

If the script contains `main` it will be ran after all the script is executed.
It also accepts arguments from the command line.
You can run it like this: `nu foo.nu arg1 --flag` of if the script contains a hashbang line (`#!/usr/bin/env nu`): `./foo.nu arg1 --flag`.
