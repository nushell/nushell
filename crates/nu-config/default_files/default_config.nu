# Built-in default config file.
#
# All `$env.config` defaults (including color_config, menus, and Nushell menu
# keybindings) now live in Rust `Config::default()` so they are visible under
# `nu -n` / `nu -n --no-std-lib` via `$env.config`.
#
# This file is still evaluated before the user's `config.nu` during interactive
# startup so users can rely on the load order. Leave it empty (or assign only
# intentional overrides) unless you need Nu-side default setup.
#
# version = "0.115.1"
$env.config = {}
