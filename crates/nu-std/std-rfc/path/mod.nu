# Helper function for `path with` commands
def with-field [field: string, value: string] {
  path parse
  | update $field $value
  | path join
}

# Replace extension of input file paths.
#
# Note that it doesn't change the file name locally.
#
# # Example
# - setting path ext to `rs`
# ```nushell
# > "ab.txt" | path with-extension "rs"
# ab.rs
# > "ab.txt" | path with-extension ".rs"
# ab.rs
#
# - setting a list of input path ext to `rs`
# > ["ab.txt", "cd.exe"] | path with-extension "rs"
# ╭───┬──────────╮
# │ 0 │ ab.rs    │
# │ 1 │ cd.rs    │
# ╰───┴──────────╯
# ```
export def with-extension [ext: string] {
  let path = $in
  let ext_trim = if $ext starts-with "." {
    $ext | str substring 1..
  } else {
    $ext
  }
  $path | with-field extension $ext_trim
}

# Replace stem of input file paths.
#
# Note that it doesn't change the file name locally.
#
# # Example
# - replace stem with "share"
# ```nushell
# > "/usr/bin" | path with-stem "share"
# /usr/share
#
# - replace stem with "nushell"
# > ["/home/alice/", "/home/bob/secret.txt"] | path with-stem "nushell"
# ╭───┬───────────────────────╮
# │ 0 │ /home/nushell         │
# │ 1 │ /home/bob/nushell.txt │
# ╰───┴───────────────────────╯
# ```
export def with-stem [stem: string] { with-field stem $stem }

# Replace parent field of input file paths.
#
# # Example
# - replace parent path with `/usr/share`
# ```nushell
# > "/etc/foobar" | path with-parent "/usr/share/"
# /usr/share/foobar
#
# - replace parent path with `/root/` for all filenames in list
# > ["/home/rose/meow", "/home/fdncred/"] | path with-parent "/root/"
# ╭───┬───────────────╮
# │ 0 │ /root/meow    │
# │ 1 │ /root/fdncred │
# ╰───┴───────────────╯
# ```
export def with-parent [parent: string] { with-field parent $parent }
