#!/bin/sh

set -e

if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "x86_64" ]; then
    target="x86_64-apple-darwin"
elif [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "x86_64" ]; then
    target="x86_64-unknown-linux-musl"
elif [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "aarch64" ]; then
    target="aarch64-unknown-linux-musl"
elif [ "$(uname -s)" = "Linux" ] && ( uname -m | grep -q -e '^arm' ); then
    target="arm-unknown-linux-gnueabihf"
else
    echo "Unsupported OS or architecture"
    exit 1
fi

fetch()
{
    if command -v curl > /dev/null; then
        if [ "$#" -eq 2 ]; then curl -sSL -o "$1" "$2"; else curl -sSL "$1"; fi
    elif command -v wget > /dev/null; then
        if [ "$#" -eq 2 ]; then wget -O "$1" "$2"; else wget -nv -O - "$1"; fi
    else
        echo "Can't find curl or wget, can't download package"
        exit 1
    fi
}

echo "Detected target: $target"

url=$(
    fetch https://api.github.com/repos/nushell/nushell/releases/latest \
    | grep browser_download_url \
    | grep musl \
    | cut -f4 -d '"' \
    || true
)
if ! test "$url"; then
    echo "Could not find release info"
    exit 1
fi

echo "Downloading nu..."

temp_dir=$(mktemp -dt xh.XXXXXX)
trap 'rm -rf "$temp_dir"' EXIT INT TERM
cd "$temp_dir"

if ! fetch nu.tar.gz "$url"; then
    echo "Could not download tarball"
    exit 1
fi

user_bin="$HOME/.local/bin"
case $PATH in
    *:"$user_bin":* | "$user_bin":* | *:"$user_bin")
        default_bin=$user_bin
        ;;
    *)
        default_bin='/usr/local/bin'
        ;;
esac

_read_installdir() {
    printf "Install location [default: %s]: " "$default_bin"
    read -r nu_installdir < /dev/tty
    nu_installdir=${nu_installdir:-$default_bin}
}

if [ -z "$NU_BINDIR" ] && [ -t 1 ]; then
    _read_installdir
    while ! test -d "$nu_installdir"; do
        echo "Directory $nu_installdir does not exist"
        _read_installdir
    done
else
    nu_installdir=${NU_BINDIR:=$default_bin}
fi

tar xzf nu.tar.gz

if test -w "$nu_installdir" || [ -n "$NU_BINDIR" ]; then
    mv nu-*/nu* "$nu_installdir/"
else
    sudo mv nu-*/nu* "$nu_installdir/"
fi

echo "nu $("$nu_installdir"/nu -v) has been installed to: $nu_installdir/nu
If you want to set nu as your shell run following commands:
echo $nu_installdir/nu | sudo tee -a /etc/shells
chsh -s $nu_installdir/nu"
