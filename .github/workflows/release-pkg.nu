#!/usr/bin/env nu

# Created: 2022/05/26 19:05:20
# Description:
#   A script to do the github release task, need nushell to be installed.
# REF:
#   1. https://github.com/volks73/cargo-wix

# The main binary file to be released
let bin = 'nu'
let os = $env.OS
let target = $env.TARGET
# Repo source dir like `/home/runner/work/nushell/nushell`
let src = $env.GITHUB_WORKSPACE
let flags = $env.TARGET_RUSTFLAGS
let dist = $'($env.GITHUB_WORKSPACE)/output'
let version = (open Cargo.toml | get package.version)

# $env

$'(char nl)Packaging ($bin) v($version) for ($target) in ($src)...'; hr-line -b
if not ('Cargo.lock' | path exists) { cargo generate-lockfile }

$'Start building ($bin)...'; hr-line

# ----------------------------------------------------------------------------
# Build for Ubuntu and macOS
# ----------------------------------------------------------------------------
if $os in ['ubuntu-latest', 'macos-latest'] {
    if $os == 'ubuntu-latest' {
        sudo apt-get install libxcb-composite0-dev -y
    }
    if $target == 'aarch64-unknown-linux-gnu' {
        sudo apt-get install gcc-aarch64-linux-gnu -y
        let-env CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
        cargo-build-nu $flags
    } else if $target == 'armv7-unknown-linux-gnueabihf' {
        sudo apt-get install pkg-config gcc-arm-linux-gnueabihf -y
        let-env CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER = 'arm-linux-gnueabihf-gcc'
        cargo-build-nu $flags
    } else {
        # musl-tools to fix 'Failed to find tool. Is `musl-gcc` installed?'
        # Actually just for x86_64-unknown-linux-musl target
        if $os == 'ubuntu-latest' { sudo apt install musl-tools -y }
        cargo-build-nu $flags
    }
}

# ----------------------------------------------------------------------------
# Build for Windows without static-link-openssl feature
# ----------------------------------------------------------------------------
if $os in ['windows-latest'] {
    if ($flags | str trim | is-empty) {
        cargo build --release --all --target $target --features=extra
    } else {
        cargo build --release --all --target $target --features=extra $flags
    }
}

# ----------------------------------------------------------------------------
# Prepare for the release archive
# ----------------------------------------------------------------------------
let suffix = if $os == 'windows-latest' { '.exe' }
# nu, nu_plugin_* were all included
let executable = $'target/($target)/release/($bin)*($suffix)'
$'Current executable file: ($executable)'

cd $src; mkdir $dist;
rm -rf $'target/($target)/release/*.d' $'target/($target)/release/nu_pretty_hex*'
$'(char nl)All executable files:'; hr-line
ls -f $executable

$'(char nl)Copying release files...'; hr-line
cp -v README.release.txt $'($dist)/README.txt'
[LICENSE $executable] | each {|it| cp -rv $it $dist } | flatten

$'(char nl)Check binary release version detail:'; hr-line
let ver = if $os == 'windows-latest' {
    (do -i { ./output/nu.exe -c 'version' }) | str join
} else {
    (do -i { ./output/nu -c 'version' }) | str join
}
if ($ver | str trim | is-empty) {
    $'(ansi r)Incompatible nu binary...(ansi reset)'
} else { $ver }

# ----------------------------------------------------------------------------
# Create a release archive and send it to output for the following steps
# ----------------------------------------------------------------------------
cd $dist; $'(char nl)Creating release archive...'; hr-line
if $os in ['ubuntu-latest', 'macos-latest'] {

    $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls

    let archive = $'($dist)/($bin)-($version)-($target).tar.gz'
    tar czf $archive *
    print $'archive: ---> ($archive)'; ls $archive
    echo $'::set-output name=archive::($archive)'

} else if $os == 'windows-latest' {

    let releaseStem = $'($bin)-($version)-($target)'

    $'(char nl)Download less related stuffs...'; hr-line
    aria2c https://github.com/jftuga/less-Windows/releases/download/less-v590/less.exe -o less.exe
    aria2c https://raw.githubusercontent.com/jftuga/less-Windows/master/LICENSE -o LICENSE-for-less.txt

    # Create Windows msi release package
    if (get-env _EXTRA_) == 'msi' {

        let wixRelease = $'($src)/target/wix/($releaseStem).msi'
        $'(char nl)Start creating Windows msi package...'
        cd $src; hr-line
        # Wix need the binaries be stored in target/release/
        cp -r $'($dist)/*' target/release/
        cargo install cargo-wix --version 0.3.3
        cargo wix --no-build --nocapture --package nu --output $wixRelease
        echo $'::set-output name=archive::($wixRelease)'

    } else {

        $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls
        let archive = $'($dist)/($releaseStem).zip'
        7z a $archive *
        print $'archive: ---> ($archive)';
        let pkg = (ls -f $archive | get name)
        if not ($pkg | is-empty) {
            echo $'::set-output name=archive::($pkg | get 0)'
        }
    }
}

def 'cargo-build-nu' [ options: string ] {
    if ($options | str trim | is-empty) {
        cargo build --release --all --target $target --features=extra,static-link-openssl
    } else {
        cargo build --release --all --target $target --features=extra,static-link-openssl $options
    }
}

# Print a horizontal line marker
def 'hr-line' [
    --blank-line(-b): bool
] {
    print $'(ansi g)---------------------------------------------------------------------------->(ansi reset)'
    if $blank_line { char nl }
}

# Get the specified env key's value or ''
def 'get-env' [
    key: string           # The key to get it's env value
    default: string = ''  # The default value for an empty env
] {
    $env | get -i $key | default $default
}
