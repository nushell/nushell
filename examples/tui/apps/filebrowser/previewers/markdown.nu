# Markdown previewer for examples/tui/apps/filebrowser/filebrowser.nu: rendered by `glow`.
# The built-in text keys scroll it; `n` and `p` jump between headings.

$env.file-browser.preview.plugins ++= [{
    name: markdown
    handles: {|file| $file.ext == "md" }
    # Fed through stdin: given a path, glow waits on a stdin that is not a
    # terminal, and inside the TUI it never is.
    render: {|file, ctx|
        open --raw $file.path
        | glow --style dark --width $env.TUI_WIDTH
        | absolute-links ($file.path | path dirname)
    }
    keys: {
        n: {|k| { action: scroll, id: view, line: (heading-after $k.file $k.view) } }
        p: {|k| { action: scroll, id: view, line: (heading-before $k.file $k.view) } }
    }
    hint: "n/p: heading  arrows/pgup/pgdn: scroll"
}]

# glow keeps each link's href as written, and a terminal opens an OSC 8
# link as is, so a relative one (`README.md`) goes nowhere. Hrefs without
# a scheme become `file://` URLs: absolute paths as they are, the rest
# under `dir`.
def absolute-links [dir: string]: string -> string {
    const OSC8 = "\u{1b}\\]8;([^;]*);"
    const END = "(\u{7}|\u{1b}\\\\)"
    $in
    | str replace --all -r $"($OSC8)\(/[^:\u{7}\u{1b}]*\)($END)" "\u{1b}]8;$1;file://$2$3"
    | str replace --all -r $"($OSC8)\([^:/\u{7}\u{1b}][^:\u{7}\u{1b}]*\)($END)" ("\u{1b}]8;$1;file://" + $dir + "/$2$3")
}

# Lines of the rendered text where the source's headings land. glow styles
# headings rather than keeping their `#`s (some levels keep them), so they
# are found by their text, with any leading `#`s ignored on both sides.
def heading-lines [file: record, view: record]: nothing -> list<int> {
    let title = { str trim | str replace -r '^#+\s*' '' | str trim }
    let titles = open --raw $file.path | lines | where $it =~ '^#{1,6} ' | each $title
    let rendered = $view.text | ansi strip | lines | each $title
    $titles
    | each {|title| $rendered | enumerate | where item == $title | get -o 0.index }
    | compact
    | uniq
}

def heading-after [file: record, view: record]: nothing -> int {
    heading-lines $file $view | where $it > $view.scroll | get -o 0 | default $view.scroll
}

def heading-before [file: record, view: record]: nothing -> int {
    heading-lines $file $view | where $it < $view.scroll | last | default 0
}
