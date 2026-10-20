#!/usr/bin/env nu
# Every slot `tui theme` takes, on one screen that uses them all, in two
# complete themes. The same record, set as `$env.config.tui`, themes every
# `tui` app; `tui theme` lays it over the config for one app only.
#
#   nu examples/tui/theme/showcase.nu            # Nord
#   nu examples/tui/theme/showcase.nu gruvbox
#
# Keys
#   t              next theme (the TUI is rebuilt: a theme is fixed at `tui run`)
#   tab            move focus between the widgets: watch `border_focused`
#   /              the search box; what matches is drawn in `highlight`
#   space          check a row in the table: the marks use `highlight` too
#   left/right     switch tabs: `tab_active` and `tab_inactive`
#   q, esc         quit
#
# Slots, and what each colors
#   surface            fill behind every widget (only `bg` is read)
#   backdrop           fill behind a dialog, such as the key panel (only `bg`)
#   title_bar          `tui label --titlebar`, and a dialog's title
#   status_bar         `tui label --status`, with the focus and filter hints
#   border             the frame round a widget
#   border_focused     the frame round the focused widget
#   border_type        the frame's lines: a `table --theme` name
#   header             a table's column names
#   selected           the highlighted row of a table, tree, select, or
#                      menu, and a focused button
#   highlight          search matches, `[x]` marks, keys in the key panel
#   muted              placeholders, `(loading...)`, dimmed hints
#   tab_active         the open tab's title
#   tab_inactive       the other tabs' titles
#   progress           the reached part of `tui progress`'s bar (`muted` beyond)
#   button             `tui button`
#   file_dir           a path cell by what the file is: a directory,
#   file_orphan        a symlink whose target is gone (`bg` reads best),
#   file_exec          an executable, an image, audio or video, an archive,
#   file_image         a PDF or Word document. Any other file is plain text.
#   file_media         Path cells are `ls`'s `name` beside `type`, a
#   file_archive       `tui table --color <column>`, and tree nodes.
#   file_document
#
# A color is what `$env.config.color_config` takes: a color name, `#rrggbb`,
# or `{fg, bg, attr}` where `attr` is letters from `b`old, `d`im,
# `i`talic, `u`nderline, `r`everse.
#
# Glyph slots, strings rather than colors. The caps are `{open, close}`,
# one cell each, drawn in the text's fill color, so with powerline's
# half-circles (a Nerd Font) a bar becomes a pill, as yazi's mode and tabs:
#   title_caps         round the title bar's text (the bar no longer fills)
#   status_caps        round each status segment
#   tab_caps           round each tab's title (give the tab slots a `bg`)
#   selected_caps      beside the highlighted row of a table, select, tree
#   check              `{on, off}` marks of `--multi` rows, one width
#   radio              `{on, off}` marks of `tui select` rows
#   tree_open          the arrows before a tree node with children
#   tree_closed
#   progress_fill      the glyph `tui progress`'s bar is made of

# Each theme fills every slot. Nord: https://www.nordtheme.com/docs/colors-and-palettes
# Gruvbox: https://github.com/morhetz/gruvbox (the dark palette).
const THEMES = {
    nord: {
        surface: { bg: "#2e3440" }                      # nord0
        backdrop: { bg: "#3b4252" }                     # nord1
        title_bar: { fg: "#eceff4", bg: "#5e81ac", attr: b }
        status_bar: { fg: "#d8dee9", bg: "#3b4252" }
        border: { fg: "#4c566a" }                       # nord3
        border_focused: { fg: "#88c0d0" }               # nord8
        border_type: rounded
        header: { fg: "#8fbcbb", attr: b }              # nord7
        selected: { fg: "#eceff4", bg: "#434c5e", attr: b }
        highlight: { fg: "#ebcb8b", attr: b }           # nord13
        muted: { fg: "#4c566a" }
        tab_active: { fg: "#eceff4", bg: "#5e81ac", attr: b }   # caps need a bg
        tab_inactive: { fg: "#d8dee9", bg: "#3b4252" }
        progress: { fg: "#a3be8c" }                     # nord14
        button: { fg: "#2e3440", bg: "#81a1c1" }        # on nord9
        file_dir: { fg: "#81a1c1" }                     # nord9
        file_orphan: { bg: "#bf616a" }                  # nord11
        file_exec: { fg: "#a3be8c" }                    # nord14
        file_image: { fg: "#ebcb8b" }                   # nord13
        file_media: { fg: "#b48ead" }                   # nord15
        file_archive: { fg: "#d08770" }                 # nord12
        file_document: { fg: "#8fbcbb" }                # nord7
        # Powerline's half circles, U+E0B6 and U+E0B4 (a Nerd Font): the
        # pills yazi's bars and tabs are drawn with.
        title_caps: { open: "\u{e0b6}", close: "\u{e0b4}" }
        status_caps: { open: "\u{e0b6}", close: "\u{e0b4}" }
        tab_caps: { open: "\u{e0b6}", close: "\u{e0b4}" }
        selected_caps: { open: "\u{e0b6}", close: "\u{e0b4}" }
        check: { on: "▍", off: " " }                    # yazi's marker bar
        radio: { on: "◉", off: "○" }
        tree_open: "▼"
        tree_closed: "▶"
        progress_fill: "━"
    }
    gruvbox: {
        surface: { bg: "#282828" }                      # dark0
        backdrop: { bg: "#3c3836" }                     # dark1
        title_bar: { fg: "#282828", bg: "#fabd2f", attr: b }
        status_bar: { fg: "#ebdbb2", bg: "#3c3836" }
        border: { fg: "#665c54" }                       # dark3
        border_focused: { fg: "#fe8019" }               # orange
        border_type: heavy
        header: { fg: "#8ec07c", attr: b }              # aqua
        selected: { fg: "#ebdbb2", bg: "#504945", attr: b }
        highlight: { fg: "#fabd2f", attr: b }           # yellow
        muted: { fg: "#928374" }                        # gray
        tab_active: { fg: "#282828", bg: "#fe8019", attr: b }
        tab_inactive: { fg: "#ebdbb2", bg: "#3c3836" }
        progress: { fg: "#b8bb26" }                     # green
        button: { fg: "#282828", bg: "#83a598" }        # on blue
        file_dir: { fg: "#83a598" }                     # blue
        file_orphan: { bg: "#fb4934" }                  # red
        file_exec: { fg: "#b8bb26" }                    # green
        file_image: { fg: "#fabd2f" }                   # yellow
        file_media: { fg: "#d3869b" }                   # purple
        file_archive: { fg: "#fb4934" }                 # red
        file_document: { fg: "#8ec07c" }                # aqua
        # Plain Unicode half-circles: no special font needed.
        title_caps: { open: "◖", close: "◗" }
        tab_caps: { open: "◖", close: "◗" }
        selected_caps: { open: "◖", close: "◗" }
        check: { on: "■", off: "□" }
        radio: { on: "●", off: "○" }
        tree_open: "⊟"
        tree_closed: "⊞"
        progress_fill: "▰"
    }
}

# One run under `name`. Returns `tui run`'s record; `t` submits `next`.
def show [name: string, dir: path]: nothing -> record {
    let theme = $THEMES | get -o $name
    if $theme == null {
        error make { msg: $"no theme ($name); one of ($THEMES | columns | str join ', ')" }
    }
    ls --mime-type $dir
    | insert file { $in.name | path basename }
    | tui label --titlebar $"theme: ($name)"
    | tui tab "files" [
        (tui split --sizes [1fr 24] [
            (tui box "list" [
                (tui search --bind "/" --placeholder "type to filter")
                (tui table --id files --columns [file size type] --icons --color file --multi --focus)
            ])
            (tui split --vertical --sizes [3 1fr 3] [
                (tui progress --label "used" --value 0.42)
                (tui select --title "kind" [small medium large])
                (tui button "quit" {|| { action: quit } })
            ])
        ])
    ]
    | tui tab "tree" [(tui tree --walk)]
    | tui label --status "t: next theme  tab: focus  /: search  space: check  q: quit"
    | tui theme $theme
    | tui bind t {|| { action: submit, selected: next } }
    | tui run
}

# `t` rebuilds the screen with the next theme; anything else ends it.
def main [name: string = nord, dir: path = "."] {
    let names = $THEMES | columns
    mut name = $name
    loop {
        let out = show $name $dir
        if $out.action != submit { break }
        let at = $names | enumerate | where item == $name | get -o 0.index | default 0
        $name = $names | get (($at + 1) mod ($names | length))
    }
}
