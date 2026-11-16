#!/usr/bin/env nu
# A paged grid of thumbnails, built from `tui preview` and `tui image`.
#
#   nu examples/tui/image/gallery.nu                 # the images in this directory
#   nu examples/tui/image/gallery.nu ~/Pictures
#   nu examples/tui/image/gallery.nu --protocol halfblocks    # or none, kitty, ...
#
#   Enter on a focused thumbnail returns what the TUI knows of its image,
#   the pane's `values.<id>.image`: `{width, height, format, protocol, ...}`.
#
# How it works
#   The grid is six `tui preview` cells that all follow the `pages` table:
#   cell `i` shows the i-th image of the highlighted page. Moving to another
#   page runs every cell again, and each draws its new image where the old
#   one was (under Kitty, a pane keeps one image id, and sending a new image
#   under it replaces the last), so nothing has to be cleared between pages.
#   The cells read with `--threads`, so the table keeps moving while they load.
#
#   The logo under the table is `tui image`, which shows one fixed image and
#   follows nothing, capped with `--image-width` and `--image-height`.
#
#   Pictures use real pixels on Kitty, Sixel, and iTerm2 terminals, and
#   colored half blocks elsewhere; `--protocol` sets
#   `$env.config.tui.image_protocol` to force one, or `none` to describe
#   each image in text instead.
#
# Keys
#   n/p, up/down     next or previous page
#   tab              focus a thumbnail: `+`/`-` zoom it, arrows pan, `0` resets
#   enter            on a thumbnail, quit and return its image record
#   q, esc           quit

const LOGO = path self ../../../assets/icons/nushell-original.png
const COLUMNS = 3
const ROWS = 2

# The formats `tui image` decodes (the `image` features in Cargo.toml).
# `ls --mime-type` would also pass HEIC, AVIF, camera raw, and others it
# cannot read, so the listing is filtered by extension instead.
const EXTENSIONS = [png jpg jpeg gif webp bmp ico tif tiff]

# One grid cell: the `i`-th image of the highlighted page, or nothing.
# Builders take what is piped in as data for the TUI, and inside `each`
# that is the item, so `null |` starts each one from nothing.
def cell [i: int]: nothing -> any {
    null | tui preview --id $"cell-($i)" --from pages --threads --image-fit fit {|page|
        let file = $page.files | get -o $i
        if $file == null { "" } else { open --raw $file }
    }
}

# The rows of cells, `COLUMNS` to a row.
def grid []: nothing -> any {
    let rows = 0..<($COLUMNS * $ROWS) | each { $in } | chunks $COLUMNS
        | each {|row| null | tui split ($row | each {|i| cell $i }) }
    null | tui split --vertical $rows
}

# `{action: select}` for the page `delta` away from the highlighted one.
def turn [state: record, delta: int, count: int]: nothing -> record {
    let index = [($state.values.pages.index + $delta) ($count - 1)] | math min
    { action: select, id: pages, index: ([$index 0] | math max) }
}

def main [
    dir: path = "."
    --protocol: string   # `$env.config.tui.image_protocol`: auto, kitty, sixel, iterm2, halfblocks, none
] {
    if $protocol != null {
        $env.config.tui.image_protocol = $protocol
    }
    let files = ls ($dir | path expand)
        | where type != dir
        | where {|f| ($f.name | path parse | get extension | str lowercase) in $EXTENSIONS }
        | get name
    if ($files | is-empty) {
        error make { msg: $"no images in ($dir)" }
    }
    let pages = $files | chunks ($COLUMNS * $ROWS) | enumerate
        | each {|p| { page: ($p.index + 1), images: ($p.item | length), files: $p.item } }
    let count = $pages | length
    $pages
    | tui label --titlebar $"gallery: ($files | length) images in ($dir | path expand)"
    | tui split --sizes [20 1fr] [
        (null | tui split --vertical --sizes [1fr 10] [
            (tui table --id pages --columns [page images] --focus)
            (tui image $LOGO --title "tui image" --image-width "80%" --image-height 6)
        ])
        (grid)
    ]
    | tui label --status "n/p: page  tab: focus a thumbnail  +/-: zoom  enter: info  q: quit"
    | tui bind n {|state| turn $state 1 $count }
    | tui bind p {|state| turn $state (-1) $count }
    | tui bind enter {|state|
        if $state.focused == null { return null }
        let image = $state.values | get -o $state.focused | get -o image
        if $image != null { { action: submit, selected: $image } }
    }
    | tui run
    | if $in.action == submit { $in.selected } else { null }
}
