#!/usr/bin/env nu
# A two-panel file browser built from the `tui` commands: a directory listing
# on the left, the highlighted entry on the right as a picture or as text.
#
#   nu examples/tui/apps/filebrowser/filebrowser.nu          # browse the current directory
#   nu examples/tui/apps/filebrowser/filebrowser.nu ~/Pictures
#
#   The picked file's row is the result. Piped, it comes out as nuon, since
#   a script's record would otherwise reach the next command as a drawn
#   table (the TUI itself draws on the terminal, not on stdout):
#
#   nu examples/tui/apps/filebrowser/filebrowser.nu | from nuon | open $in.name
#
# Previewers
#   Every row goes to the first previewer in `$env.file-browser.preview.plugins`
#   whose `handles` closure accepts it; the rest fall back to the built-in
#   pane: images draw, text shows, binaries get a note. The bundled ones live
#   in previewers/: PDF pages via poppler, SVG via resvg, Markdown via glow.
#   To add one, append a record to that list before `tui run` (sourcing a
#   file that does so is enough); previewers/pdf.nu documents the fields.
#
#   Previewers run on a thread (`tui preview --threads`): the list keeps
#   moving while a page rasterises, and leaving the row kills the renderer.
#   Pictures draw at one pixel per pixel unless larger than the pane; Kitty,
#   Sixel, and iTerm2 terminals get real pixels, others colored half blocks.
#
# Keys
#   up/down, j/k     move through the entries
#   enter            open a directory; pick a file and quit (its row is returned)
#   backspace        go up to the parent directory
#   .                show or hide dotfiles (hidden to start, as in yazi)
#   tab              switch between the list and the preview
#   f                give the preview the whole window, and back
#   m, then a key    linemode: what the list shows beside each name. `m`
#                    alone opens a panel of the choices (`tui bind "m s"`)
#   Z                zoxide's picker, full screen as in yazi, with the
#                    highlighted directory listed below; a pick goes there,
#                    Esc stays (`{action: suspend}`)
#   in the preview   the previewer's keys first (the status bar shows them),
#                    then the built-ins: arrows, PageUp/PageDown, Home/End
#                    scroll text; `+`/`-` zoom an image, arrows pan, `0` resets
#   q, esc           quit

# Defaults, under anything set before the script runs: `theme` holds slots
# laid over `$env.config.tui` (`tui theme`), so
# `$env.file-browser.theme.selected = { bg: "#3b4252" }` recolors the
# highlighted row here and nowhere else. Previewers append to `preview.plugins`.
$env.file-browser = {
    preview: {
        raster_px: 1600         # width of rasterised SVG and PDF pages
        max_text_bytes: 65536   # how much of a text file the fallback shows
        plugins: []
    }
    theme: {}
} | merge deep ($env.file-browser? | default {})
source previewers/pdf.nu
source previewers/svg.nu
source previewers/markdown.nu

# `m` then the key picks what the list shows beside the names.
const LINEMODES = [
    [key, label, columns];
    [s, size, [file size]]
    [m, modified, [file modified]]
    [c, created, [file created]]
    [p, permissions, [file mode]]
    [o, owner, [file user]]
    [t, "mime type", [file type]]
    [n, none, [file]]
]

# `ls --mime-type` reports directories as `dir`, not as a mime type.
const DIR_TYPE = "dir"

# The rows for one directory. `name` holds the full path, which is what the
# preview opens; `file` holds the bare name, which is what the table shows
# (`tui table --icons --color file` gives it its icon and file color);
# `type` is the mime type; the rest are `ls -l`'s columns for the
# linemodes. The first row is `..`, so every directory can be left again.
# The order is yazi's default: directories first, then names compared
# without regard to case. Dotfiles are left out unless `--all`.
def listing [dir: path, --all]: nothing -> table {
    let parent = { name: ($dir | path dirname), type: $DIR_TYPE, file: "..", size: null }
    ls -l --mime-type --all=$all $dir
    | insert file { $in.name | path basename }
    | sort-by -i {|row| $row.type != $DIR_TYPE } file
    | prepend $parent
    | move file --first
}

# The actions that take the browser to `dir`: the directory and the dotfile
# choice live in the TUI's context (`tui context`), the rows are the data.
def goto [state: record, dir: path]: nothing -> list {
    [
        { action: context, value: { dir: $dir } }
        { action: data, value: (listing $dir --all=$state.context.hidden) }
    ]
}

# What previewers are given.
def file-info [row: record]: nothing -> record {
    {
        path: $row.name
        name: ($row.name | path basename)
        ext: (($row.name | path parse).extension | str lowercase)
        mime: $row.type
        type: (if $row.type == $DIR_TYPE { "dir" } else { "file" })
        size: $row.size
    }
}

# The first previewer that takes the file, or null.
def plugin-for [file: record]: nothing -> any {
    $env.file-browser.preview.plugins | where {|p| do $p.handles $file } | get -o 0
}

# The context a new file starts with: the previewer's own, plus its hint
# for the status bar.
def initial-context [row: record]: nothing -> record {
    let plugin = plugin-for (file-info $row)
    if $plugin == null { {} } else {
        $plugin.context? | default {} | merge (if $plugin.hint? == null { {} } else { { hint: $plugin.hint } })
    }
}

# What the pane shows: the previewer's render, else the built-in path.
def preview [row: record, ctx: record]: nothing -> any {
    if $row.type == $DIR_TYPE {
        return $"($row.name)/\n\(directory\)"
    }
    let file = file-info $row
    let plugin = plugin-for $file
    if $plugin != null {
        return (do $plugin.render $file $ctx)
    }
    if ($file.mime | str starts-with "image/") {
        return (open --raw $row.name)
    }
    let head = open --raw $row.name | bytes at ..$env.file-browser.preview.max_text_bytes
    if ($head | bytes index-of 0x[00]) >= 0 {
        $"($row.name)\n\(binary file, ($row.size)\)"
    } else {
        $head | decode utf-8
    }
}

# The highlighted file row. `$state.selected` follows focus, so with the
# preview focused it would be the pane's value instead.
def current-row [state: record]: nothing -> any {
    $state.values.files.row
}

# `Z`: zoxide's interactive picker, which is fzf, laid out as yazi's: the
# whole screen, centred by its margins, the highlighted directory listed in
# a preview window below. The bind names the layout as the area only so the
# TUI's own screen stays (no flash of the shell); fzf takes all of it. A pick
# lists that directory; exit code 130 is Esc.
def zoxide-pick [state: record]: nothing -> any {
    let opts = [
        --height=100% "--margin=5%,15%" --border --layout=reverse --info=inline
        "--preview-window=down,30%,sharp" "--preview='\\command -p ls -Cp {2..}'"
        --exact --no-sort --cycle --keep-right --exit-0
        "--bind=ctrl-z:ignore,btab:up,tab:down"
    ] | str join " "
    let picked = with-env { _ZO_FZF_OPTS: $opts, CLICOLOR: 1, CLICOLOR_FORCE: 1 } {
        zoxide query --interactive --exclude $state.context.dir | complete
    }
    match $picked.exit_code {
        0 => (goto $state ($picked.stdout | str trim))
        130 => null
        _ => (error make { msg: $"zoxide: ($picked.stderr | str trim)" })
    }
}

# A key while the preview is focused: the previewer's handler, if it has
# one. A bare record is a context patch; an action record passes through;
# null lets the key go on to the pane's built-ins.
def plugin-key [key: string, state: record]: nothing -> any {
    let file = file-info (current-row $state)
    let plugin = plugin-for $file
    if $plugin == null { return null }
    let handler = $plugin | get -o keys | get -o $key
    if $handler == null { return null }
    let out = do $handler { ctx: $state.values.view.context, file: $file, view: $state.values.view }
    if $out == null or ($out | describe -d).type == list or ($out | get -o action) != null {
        $out
    } else {
        { action: context, id: view, value: $out }
    }
}

# `m <key>` for each linemode, labelled for the key panel.
def linemode-binds []: any -> any {
    let app = $in
    $LINEMODES | reduce --fold $app {|mode, app|
        $app | tui bind $"m ($mode.key)" --label $"Linemode: ($mode.label)" {||
            { action: columns, id: files, columns: $mode.columns }
        }
    }
}

# One scoped bind per key any previewer handles, each dispatching to the
# previewer of the highlighted file when pressed.
def plugin-binds []: any -> any {
    let app = $in
    let keys = $env.file-browser.preview.plugins | each { $in.keys? | default {} | columns } | flatten | uniq
    $keys | reduce --fold $app {|key, app|
        $app | tui bind --when view $key {|state| plugin-key $key $state }
    }
}

def main [dir: path = "."] {
    let dir = $dir | path expand
    listing $dir
    | tui label --titlebar "file browser"
    | tui theme $env.file-browser.theme
    | tui context { dir: $dir, hidden: false }
    | tui split --id layout --sizes [40% 1fr] [
        (tui table --id files --columns [file size] --icons --color file --focus)
        (tui preview --id view --threads --on-change {|s| initial-context $s.values.view.row } {|row, ctx| preview $row $ctx })
    ]
    | tui label --status "enter: open / pick  backspace: up  .: dotfiles  tab: preview  f: full  m: linemode  Z: zoxide"
    | plugin-binds
    | linemode-binds
    | tui bind f {|state|
        # Collapse the list to give the preview the window, or restore it.
        if ($state.values.layout | first) == 0 {
            [{ action: resize, id: layout, sizes: [40% 1fr] } { action: focus, id: files }]
        } else {
            [{ action: resize, id: layout, sizes: [0 1fr] } { action: focus, id: view }]
        }
    }
    | tui bind backspace {|state|
        # Up one level, with the directory just left highlighted.
        let from = $state.context.dir
        let up = goto $state ($from | path dirname)
        let index = $up.1.value | enumerate | where item.name == $from | get -o 0.index | default 0
        $up | append { action: select, id: files, index: $index }
    }
    | tui bind "." {|state|
        # Show or hide dotfiles, keeping the highlighted entry if it stays.
        let hidden = not $state.context.hidden
        let rows = listing $state.context.dir --all=$hidden
        let name = (current-row $state).name
        let index = $rows | enumerate | where item.name == $name | get -o 0.index | default 0
        [
            { action: context, value: { hidden: $hidden } }
            { action: data, value: $rows }
            { action: select, id: files, index: $index }
        ]
    }
    | tui bind shift+z {|| { action: suspend, area: layout, hook: {|state| zoxide-pick $state } } }
    | tui bind enter {|state|
        # A directory replaces the rows; a file is the result.
        let row = current-row $state
        if ($row | describe -d).type != record {
            null
        } else if $row.type == $DIR_TYPE {
            goto $state $row.name
        } else {
            { action: submit, selected: $row }
        }
    }
    | tui run
    | if $in.action == submit { $in.selected } else { null }
    | if (is-terminal --stdout) { $in } else { $in | to nuon }
}
