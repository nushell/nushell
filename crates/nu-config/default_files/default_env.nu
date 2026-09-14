# Built-in default environment file.
#
# Nothing is set here any more. The `$env.config` defaults live in Rust
# `Config::default()`, and the default prompts are evaluated from `nu-cli`
# before this file, since a closure needs a parsed block. See them with
# `view source $env.config.prompt.left`.
#
# This file is still evaluated before the user's `env.nu` during startup so
# users can rely on the load order. Leave it empty (or assign only intentional
# overrides) unless you need Nu-side default setup.
#
# version = "0.115.2"
