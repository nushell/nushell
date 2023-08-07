---
title: Nushell {{VERSION}}
author: The Nu Authors
author_site: https://twitter.com/nu_shell
author_image: https://www.nushell.sh/blog/images/nu_logo.png
excerpt: Today, we're releasing version {{VERSION}} of Nu. This release adds...
---

# Nushell {{VERSION}}

Nushell, or Nu for short, is a new shell that takes a modern, structured approach to your command line. It works seamlessly with the data from your filesystem, operating system, and a growing number of file formats to make it easy to build powerful command line pipelines.

Today, we're releasing version {{VERSION}} of Nu. This release adds...

<!-- more -->

# Where to get it

Nu {{VERSION}} is available as [pre-built binaries](https://github.com/nushell/nushell/releases/tag/{{VERSION}}.0) or from [crates.io](https://crates.io/crates/nu). If you have Rust installed you can install it using `cargo install nu`.

NOTE: The optional dataframe functionality is available by `cargo install nu --features=dataframe`.

As part of this release, we also publish a set of optional plugins you can install and use with Nu. To install, use `cargo install nu_plugin_<plugin name>`.

# Themes of this release / New features

## New theme ([author](https://github.com/nushell/nushell/pulls))

# Breaking changes
<!-- TODO
    paste the output of
    ```nu
    ./make_release/release-note/list-merged-prs nushell/nushell --label breaking-change --pretty --no-author
    ```
    here
-->

# Full changelog
<!-- TODO
    paste the output of
    ```nu
    ./make_release/release-note/get-full-changelog
    ```
    here
-->
