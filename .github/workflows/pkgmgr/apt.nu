const file_control = `Package: nushell
Version: %VERSION%
Section: shells
Priority: optional
Maintainer: Nushell Project <contact@nushell.sh>
Homepage: https://nushell.sh
Vcs-Git: https://github.com/nushell/nushell.git
Vcs-Browser: https://github.com/nushell/nushell
Architecture: amd64
Depends: libssl-dev, pkg-config
Description: A modern shell for the GitHub era
 The goal of this project is to take the Unix
 philosophy of shells, where pipes connect simple
 commands together, and bring it to the modern
 style of development.
`

const file_copyright = `Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: nushell
Source: https://github.com/nushell/nushell

Files: *
Copyright: %YEAR% Nushell Project
           %YEAR% Nushell Project
License: MIT

Files: DEBIAN/*
Copyright: %YEAR% Nushell Project
           %YEAR% Nushell Project
License: MIT

License: MIT
 MIT License

 Copyright (c) 2019 - %YEAR% The Nushell Project Developers

 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:

 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.

 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 SOFTWARE.
`

const file_postinst = `#! /bin/bash

if [ "$1" = configure ] && which add-shell >/dev/null
then
    add-shell /usr/local/bin/nu
fi

exit 0
`

const file_postrm = '#!/bin/sh

set -e

case "$1" in
    upgrade|failed-upgrade|abort-install|abort-upgrade)
	;;
    remove|purge|disappear)
	if which remove-shell >/dev/null && [ -f /etc/shells ]; then
	    remove-shell /usr/local/bin/nu
	fi
	;;
    *)
        echo "postrm called with unknown argument \`$1\`" >&2
        exit 1
	;;
esac
'

# Create a .deb file containing the nushell binaries and return a string with the path to the archive
export def create-deb [
    --version: string, # the version of nushell to create a .deb file for
    --name: string, # name of the archive
    --files: list<string> # list of files to package
] nothing -> string {
    let tmpdir = "/tmp"
    let base_dir = $tmpdir | path join $name # /tmp/nushell_x.y.z
    let debian_dir = $tmpdir | path join $"($name)/DEBIAN" # /tmp/nushell_x.y.z/DEBIAN
    let bin_dir = $base_dir | path join "usr/local/bin" # /tmp/nushell_x.y.z/usr/local/bin
    let year = date now | date to-record | get year


    if not ($bin_dir | path exists) {
        print $"deb: Creating directory ($bin_dir)"
        mkdir $bin_dir
    }

    if not ($debian_dir | path exists) {
        print $"deb: Creating directory ($debian_dir)"
        mkdir $debian_dir
    }

    print "Rendering templates"
    $file_control | str replace -a "%VERSION%" $version | save -f ($debian_dir | path join "control")
    $file_copyright | str replace -a "%YEAR%" $"($year)" | save -f ($debian_dir | path join "copyright")
    $file_postinst | save -f ($debian_dir | path join "postinst")
    $file_postrm | save -f ($debian_dir | path join "postrm")

    chmod 0755 ($debian_dir | path join "postinst")
    chmod 0755 ($debian_dir | path join "postrm")

    print $"deb: copying binaries to ($bin_dir)"
    $files | each {|file| cp $file $bin_dir}

    print $"deb: building debian package ($base_dir).deb"
    ^dpkg-deb --build $base_dir

    print "deb: dpkg results"
    do {^dpkg-deb --info $"($base_dir).deb"} | complete | print
    do {^dpkg-deb --contents $"($base_dir).deb"} | complete | print

    print "deb: debian packaging complete"

    return $"($base_dir).deb"
}
