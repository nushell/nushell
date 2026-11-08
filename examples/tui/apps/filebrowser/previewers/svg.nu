# SVG previewer for examples/tui/apps/filebrowser/filebrowser.nu: rasterised by `resvg` at
# `$env.file-browser.preview.raster_px` wide, so zooming in has detail. No
# keys of its own: the built-in zoom and pan apply.

$env.file-browser.preview.plugins ++= [{
    name: svg
    # Without resvg the built-in pane takes over.
    handles: {|file| $file.ext == "svg" and (which resvg | is-not-empty) }
    render: {|file, ctx|
        resvg --width $env.file-browser.preview.raster_px $file.path -c
        | complete
        | if $in.exit_code == 0 { $in.stdout } else { $"\(resvg failed\)\n($in.stderr)" }
    }
}]
