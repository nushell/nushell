# Helper function for `path with` commands
def with-field [field: string, value: string] {
  path parse
  | update $field $value
  | path join
}

alias "path with-extension" = with-extension

# Replace extension of input file paths.
#
# Note that it doesn't change the file name locally.
@example "setting path ext to `rs`" {
    "ab.txt" | path with-extension "rs"
} --result ab.rs
@example "leading dot can be included" {
    "ab.txt" | path with-extension ".rs"
} --result ab.rs
@example "setting a list of input path ext to `rs`" {
    ["ab.txt", "cd.exe"] | path with-extension "rs"
} --result [ab.rs, cd.rs]
export def with-extension [ext: string] {
  let path = $in
  let ext_trim = if $ext starts-with "." {
    $ext | str substring 1..
  } else {
    $ext
  }
  $path | with-field extension $ext_trim
}

alias "path with-stem" = with-stem

# Replace stem of input file paths.
#
# Note that it doesn't change the file name locally.
@example "replace stem with 'share'" {
    "/usr/bin" | path with-stem "share"
} --result /usr/share
@example "replace stem with 'nushell'" {
    ["/home/alice/", "/home/bob/secret.txt"] | path with-stem "nushell"
} --result [/home/nushell, /home/bob/nushell.txt]
export def with-stem [stem: string] { with-field stem $stem }

alias "path with-parent" = with-parent

# Replace parent field of input file paths.
@example "replace parent path with `/usr/share`" {
    "/etc/foobar" | path with-parent "/usr/share/"
} --result "/usr/share/foobar"
@example "replace parent path with `/root/` for all filenames in list" {
    ["/home/rose/meow", "/home/fdncred/"] | path with-parent "/root/"
} --result [/root/meow, /root/fdncred]
export def with-parent [parent: string] { with-field parent $parent }
