#!/usr/bin/env nu

# Created: 2022/05/26 19:05:20
# Description:
#   A script to do the github release task, need nushell to be installed.
# REF:
#   1. https://github.com/volks73/cargo-wix

# Instructions for manually creating an MSI for Winget Releases when they fail
# Added 2022-11-29 when Windows packaging wouldn't work
# Updated again on 2023-02-23 because msis are still failing validation
# To run this manual for windows here are the steps I take
# checkout the release you want to publish
# 1. git checkout 0.76.0
# unset CARGO_TARGET_DIR if set (I have to do this in the parent shell to get it to work)
# 2. $env:CARGO_TARGET_DIR = ""
# 2. hide-env CARGO_TARGET_DIR
# 3. $env.TARGET = 'x86_64-pc-windows-msvc'
# 4. $env.TARGET_RUSTFLAGS = ''
# 5. $env.GITHUB_WORKSPACE = 'C:\Users\dschroeder\source\repos\forks\nushell'
# 6. $env.GITHUB_OUTPUT = 'C:\Users\dschroeder\source\repos\forks\nushell\output\out.txt'
# 7. $env.OS = 'windows-latest'
# make sure 7z.exe is in your path https://www.7-zip.org/download.html
# 8. $env.Path = ($env.Path | append 'c:\apps\7-zip')
# make sure aria2c.exe is in your path https://github.com/aria2/aria2
# 9. $env.Path = ($env.Path | append 'c:\path\to\aria2c')
# make sure you have the wixtools installed https://wixtoolset.org/
# 10. $env.Path = ($env.Path | append 'C:\Users\dschroeder\AppData\Local\tauri\WixTools')
# You need to run the release-pkg twice. The first pass, with _EXTRA_ as 'bin', makes the output
# folder and builds everything. The second pass, that generates the msi file, with _EXTRA_ as 'msi'
# 11. $env._EXTRA_ = 'bin'
# 12. source .github\workflows\release-pkg.nu
# 13. cd ..
# 14. $env._EXTRA_ = 'msi'
# 15. source .github\workflows\release-pkg.nu
# After msi is generated, you have to update winget-pkgs repo, you'll need to patch the release
# by deleting the existing msi and uploading this new msi. Then you'll need to update the hash
# on the winget-pkgs PR. To generate the hash, run this command
# 16. open target\wix\nu-0.74.0-x86_64-pc-windows-msvc.msi | hash sha256
# Then, just take the output and put it in the winget-pkgs PR for the hash on the msi


# The main binary file to be released
let bin = 'nu'
let os = $env.OS
let target = $env.TARGET
# Repo source dir like `/home/runner/work/nushell/nushell`
let src = $env.GITHUB_WORKSPACE
let flags = $env.TARGET_RUSTFLAGS
let dist = $'($env.GITHUB_WORKSPACE)/output'
let version = (open Cargo.toml | get package.version)

print $'Debugging info:'
print { version: $version, bin: $bin, os: $os, target: $target, src: $src, flags: $flags, dist: $dist }; hr-line -b

# $env

let USE_UBUNTU = 'ubuntu-20.04'

print $'(char nl)Packaging ($bin) v($version) for ($target) in ($src)...'; hr-line -b
if not ('Cargo.lock' | path exists) { cargo generate-lockfile }

print $'Start building ($bin)...'; hr-line

# ----------------------------------------------------------------------------
# Build for Ubuntu and macOS
# ----------------------------------------------------------------------------
if $os in [$USE_UBUNTU, 'macos-latest'] {
    if $os == $USE_UBUNTU {
        sudo apt update
        sudo apt-get install libxcb-composite0-dev -y
    }
    match $target {
        'aarch64-unknown-linux-gnu' => {
            sudo apt-get install gcc-aarch64-linux-gnu -y
            $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
            cargo-build-nu $flags
        }
        'riscv64gc-unknown-linux-gnu' => {
            sudo apt-get install gcc-riscv64-linux-gnu -y
            $env.CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER = 'riscv64-linux-gnu-gcc'
            cargo-build-nu $flags
        }
        'armv7-unknown-linux-gnueabihf' => {
            sudo apt-get install pkg-config gcc-arm-linux-gnueabihf -y
            $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER = 'arm-linux-gnueabihf-gcc'
            cargo-build-nu $flags
        }
        _ => {
            # musl-tools to fix 'Failed to find tool. Is `musl-gcc` installed?'
            # Actually just for x86_64-unknown-linux-musl target
            if $os == $USE_UBUNTU { sudo apt install musl-tools -y }
            cargo-build-nu $flags
        }
    }
}

# ----------------------------------------------------------------------------
# Build for Windows without static-link-openssl feature
# ----------------------------------------------------------------------------
if $os in ['windows-latest'] {
    cargo-build-nu $flags
}

# ----------------------------------------------------------------------------
# Prepare for the release archive
# ----------------------------------------------------------------------------
let suffix = if $os == 'windows-latest' { '.exe' }
# nu, nu_plugin_* were all included
let executable = $'target/($target)/release/($bin)*($suffix)'
print $'Current executable file: ($executable)'

cd $src; mkdir $dist;
rm -rf $'target/($target)/release/*.d' $'target/($target)/release/nu_pretty_hex*'
print $'(char nl)All executable files:'; hr-line
# We have to use `print` here to make sure the command output is displayed
print (ls -f $executable); sleep 1sec

print $'(char nl)Copying release files...'; hr-line
"To use Nu plugins, use the register command to tell Nu where to find the plugin. For example:

> register ./nu_plugin_query" | save $'($dist)/README.txt'
[LICENSE $executable] | each {|it| cp -rv $it $dist } | flatten
# Sleep a few seconds to make sure the cp process finished successfully
sleep 3sec

print $'(char nl)Check binary release version detail:'; hr-line
let ver = if $os == 'windows-latest' {
    (do -i { .\output\nu.exe -c 'version' }) | str join
} else {
    (do -i { ./output/nu -c 'version' }) | str join
}
if ($ver | str trim | is-empty) {
    print $'(ansi r)Incompatible nu binary...(ansi reset)'
} else { print $ver }

# ----------------------------------------------------------------------------
# Create a release archive and send it to output for the following steps
# ----------------------------------------------------------------------------
cd $dist; print $'(char nl)Creating release archive...'; hr-line
if $os in [$USE_UBUNTU, 'macos-latest'] {

    let files = (ls | get name)
    let dest = $'($bin)-($version)-($target)'
    let archive = $'($dist)/($dest).tar.gz'

    mkdir $dest
    $files | each {|it| mv $it $dest } | ignore

    print $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls $dest

    tar -czf $archive $dest
    print $'archive: ---> ($archive)'; ls $archive
    # REF: https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/
    echo $"archive=($archive)" | save --append $env.GITHUB_OUTPUT

} else if $os == 'windows-latest' {

    let releaseStem = $'($bin)-($version)-($target)'

    print $'(char nl)Download less related stuffs...'; hr-line
    aria2c https://github.com/jftuga/less-Windows/releases/download/less-v608/less.exe -o less.exe
    aria2c https://raw.githubusercontent.com/jftuga/less-Windows/master/LICENSE -o LICENSE-for-less.txt

    # Create Windows msi release package
    if (get-env _EXTRA_) == 'msi' {

        let wixRelease = $'($src)/target/wix/($releaseStem).msi'
        print $'(char nl)Start creating Windows msi package...'
        cd $src; hr-line
        # Wix need the binaries be stored in target/release/
        cp -r $'($dist)/*' target/release/
        cargo install cargo-wix --version 0.3.4
        cargo wix --no-build --nocapture --package nu --output $wixRelease
        print $'archive: ---> ($wixRelease)';
        echo $"archive=($wixRelease)" | save --append $env.GITHUB_OUTPUT

    } else {

        print $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls
        let archive = $'($dist)/($releaseStem).zip'
        7z a $archive *
        print $'archive: ---> ($archive)';
        let pkg = (ls -f $archive | get name)
        if not ($pkg | is-empty) {
            echo $"archive=($pkg | get 0)" | save --append $env.GITHUB_OUTPUT
        }
    }
}

def 'cargo-build-nu' [ options: string ] {
    if ($options | str trim | is-empty) {
        if $os == 'windows-latest' {
            cargo build --release --all --target $target
        } else {
            cargo build --release --all --target $target --features=static-link-openssl
        }
    } else {
        if $os == 'windows-latest' {
            cargo build --release --all --target $target $options
        } else {
            cargo build --release --all --target $target --features=static-link-openssl $options
        }
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
