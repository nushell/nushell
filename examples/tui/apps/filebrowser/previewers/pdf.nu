# PDF previewer for examples/tui/apps/filebrowser/filebrowser.nu: one page at a time,
# rasterised by poppler's `pdftoppm`. PgUp/PgDn and Home turn pages while
# the preview is focused; `+`/`-` and the arrows still zoom and pan.
#
# A previewer is a record in `$env.file-browser.preview.plugins`:
#   handles: {|file| bool }            file = {path, name, ext, mime, type, size}
#   context: record                    what `render` sees as $ctx for a new file
#   render:  {|file, ctx| bytes|text } PNG bytes draw as a picture
#   keys:    {<chord>: {|k| ...}}      k = {ctx, file, view}; return a context
#                                      patch, a hook action, or null to pass
#   hint:    string                    status bar text while focused

$env.file-browser.preview.plugins ++= [{
    name: pdf
    # Without poppler the built-in pane takes over.
    handles: {|file| $file.mime == "application/pdf" and (which pdftoppm | is-not-empty) }
    context: { page: 1 }
    render: {|file, ctx|
        pdftoppm -png -singlefile -f $ctx.page -l $ctx.page -scale-to $env.file-browser.preview.raster_px $file.path
        | complete
        | if $in.exit_code == 0 { $in.stdout } else { $"\(pdftoppm failed\)\n($in.stderr)" }
    }
    keys: {
        pagedown: {|k| { page: ([($k.ctx.page + 1) (pdf-pages $k.file.path)] | math min) } }
        pageup: {|k| { page: ([($k.ctx.page - 1) 1] | math max) } }
        home: {|k| { page: 1 } }
        end: {|k| { page: (pdf-pages $k.file.path) } }
    }
    hint: "pgup/pgdn/home/end: page  +/-: zoom  arrows: pan"
}]

# The page count, from `pdfinfo`; 1 when it cannot say or is not installed.
def pdf-pages [path: string]: nothing -> int {
    if (which pdfinfo | is-empty) { return 1 }
    pdfinfo $path
    | complete
    | get stdout
    | parse -r 'Pages:\s+(?<n>\d+)'
    | get -o 0.n
    | default 1
    | into int
}
