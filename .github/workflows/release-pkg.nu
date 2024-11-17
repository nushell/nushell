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
# 1. git checkout 0.86.0
# unset CARGO_TARGET_DIR if set (I have to do this in the parent shell to get it to work)
# 2. $env:CARGO_TARGET_DIR = ""
# 2. hide-env CARGO_TARGET_DIR
# 3. $env.TARGET = 'x86_64-pc-windows-msvc'
# 4. $env.GITHUB_WORKSPACE = 'D:\nushell'
# 5. $env.GITHUB_OUTPUT = 'D:\nushell\output\out.txt'
# 6. $env.OS = 'windows-latest'
# make sure 7z.exe is in your path https://www.7-zip.org/download.html
# 7. $env.Path = ($env.Path | append 'c:\apps\7-zip')
# make sure aria2c.exe is in your path https://github.com/aria2/aria2
# 8. $env.Path = ($env.Path | append 'c:\path\to\aria2c')
# make sure you have the wixtools installed https://wixtoolset.org/
# 9. $env.Path = ($env.Path | append 'C:\Users\dschroeder\AppData\Local\tauri\WixTools')
# You need to run the release-pkg twice. The first pass, with _EXTRA_ as 'bin', makes the output
# folder and builds everything. The second pass, that generates the msi file, with _EXTRA_ as 'msi'
# 10. $env._EXTRA_ = 'bin'
# 11. source .github\workflows\release-pkg.nu
# 12. cd ..
# 13. $env._EXTRA_ = 'msi'
# 14. source .github\workflows\release-pkg.nu
# After msi is generated, you have to update winget-pkgs repo, you'll need to patch the release
# by deleting the existing msi and uploading this new msi. Then you'll need to update the hash
# on the winget-pkgs PR. To generate the hash, run this command
# 15. open target\wix\nu-0.74.0-x86_64-pc-windows-msvc.msi | hash sha256
# Then, just take the output and put it in the winget-pkgs PR for the hash on the msi


# The main binary file to be released
let bin = 'nu'
let os = $env.OS
let target = $env.TARGET
# Repo source dir like `/home/runner/work/nushell/nushell`
let src = $env.GITHUB_WORKSPACE
let dist = $'($env.GITHUB_WORKSPACE)/output'
let version = (open Cargo.toml | get package.version)

print $'Debugging info:'
print { version: $version, bin: $bin, os: $os, target: $target, src: $src, dist: $dist }; hr-line -b

# $env

let USE_UBUNTU = $os starts-with ubuntu

print $'(char nl)Packaging ($bin) v($version) for ($target) in ($src)...'; hr-line -b
if not ('Cargo.lock' | path exists) { cargo generate-lockfile }

print $'Start building ($bin)...'; hr-line

# ----------------------------------------------------------------------------
# Build for Ubuntu and macOS
# ----------------------------------------------------------------------------
if $os in ['macos-latest'] or $USE_UBUNTU {
    if $USE_UBUNTU {
        sudo apt update
        sudo apt-get install libxcb-composite0-dev -y
    }
    match $target {
        'aarch64-unknown-linux-gnu' => {
            sudo apt-get install gcc-aarch64-linux-gnu -y
            $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
            cargo-build-nu
        }
        'riscv64gc-unknown-linux-gnu' => {
            sudo apt-get install gcc-riscv64-linux-gnu -y
            $env.CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER = 'riscv64-linux-gnu-gcc'
            cargo-build-nu
        }
        'armv7-unknown-linux-gnueabihf' => {
            sudo apt-get install pkg-config gcc-arm-linux-gnueabihf -y
            $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER = 'arm-linux-gnueabihf-gcc'
            cargo-build-nu
        }
        'aarch64-unknown-linux-musl' => {
            aria2c https://musl.cc/aarch64-linux-musl-cross.tgz
            tar -xf aarch64-linux-musl-cross.tgz -C $env.HOME
            $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.HOME)/aarch64-linux-musl-cross/bin')
            $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = 'aarch64-linux-musl-gcc'
            cargo-build-nu
        }
        'armv7-unknown-linux-musleabihf' => {
            aria2c https://musl.cc/armv7r-linux-musleabihf-cross.tgz
            tar -xf armv7r-linux-musleabihf-cross.tgz -C $env.HOME
            $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.HOME)/armv7r-linux-musleabihf-cross/bin')
            $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = 'armv7r-linux-musleabihf-gcc'
            cargo-build-nu
        }
        'loongarch64-unknown-linux-gnu' => {
            aria2c https://github.com/loongson/build-tools/releases/download/2024.08.08/x86_64-cross-tools-loongarch64-binutils_2.43-gcc_14.2.0-glibc_2.40.tar.xz
            tar xf x86_64-cross-tools-loongarch64-*.tar.xz
            $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.PWD)/cross-tools/bin')
            $env.CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_GNU_LINKER = 'loongarch64-unknown-linux-gnu-gcc'
            cargo-build-nu
        }
        _ => {
            # musl-tools to fix 'Failed to find tool. Is `musl-gcc` installed?'
            # Actually just for x86_64-unknown-linux-musl target
            if $USE_UBUNTU { sudo apt install musl-tools -y }
            cargo-build-nu
        }
    }
}

# ----------------------------------------------------------------------------
# Build for Windows without static-link-openssl feature
# ----------------------------------------------------------------------------
if $os in ['windows-latest'] {
    cargo-build-nu
}

# ----------------------------------------------------------------------------
# Prepare for the release archive
# ----------------------------------------------------------------------------
let suffix = if $os == 'windows-latest' { '.exe' }
# nu, nu_plugin_* were all included
let executable = $'target/($target)/release/($bin)*($suffix)'
print $'Current executable file: ($executable)'

cd $src; mkdir $dist;
rm -rf ...(glob $'target/($target)/release/*.d') ...(glob $'target/($target)/release/nu_pretty_hex*')
print $'(char nl)All executable files:'; hr-line
# We have to use `print` here to make sure the command output is displayed
print (ls -f ($executable | into glob)); sleep 1sec

print $'(char nl)Copying release files...'; hr-line
"To use the included Nushell plugins, register the binaries with the `plugin add` command to tell Nu where to find the plugin.
Then you can use `plugin use` to load the plugin into your session.
For example:

> plugin add ./nu_plugin_query
> plugin use query

For more information, refer to https://www.nushell.sh/book/plugins.html
" | save $'($dist)/README.txt' -f
[LICENSE ...(glob $executable)] | each {|it| cp -rv $it $dist } | flatten

print $'(char nl)Check binary release version detail:'; hr-line
let ver = if $os == 'windows-latest' {
    (do -i { .\output\nu.exe -c 'version' }) | str join
} else {
    (do -i { ./output/nu -c 'version' }) | str join
}
if ($ver | str trim | is-empty) {
    print $'(ansi r)Incompatible Nu binary: The binary cross compiled is not runnable on current arch...(ansi reset)'
} else { print $ver }

# ----------------------------------------------------------------------------
# Create a release archive and send it to output for the following steps
# ----------------------------------------------------------------------------
cd $dist; print $'(char nl)Creating release archive...'; hr-line
if $os in ['macos-latest'] or $USE_UBUNTU {

    let files = (ls | get name)
    let dest = $'($bin)-($version)-($target)'
    let archive = $'($dist)/($dest).tar.gz'

    mkdir $dest
    $files | each {|it| cp -v $it $dest }

    print $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls $dest | print

    tar -czf $archive $dest
    print $'archive: ---> ($archive)'; ls $archive
    # REF: https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/
    echo $"archive=($archive)" | save --append $env.GITHUB_OUTPUT

} else if $os == 'windows-latest' {

    let releaseStem = $'($bin)-($version)-($target)'

    print $'(char nl)Download less related stuffs...'; hr-line
    # todo: less-v661 is out but is released as a zip file. maybe we should switch to that and extract it?
    aria2c https://github.com/jftuga/less-Windows/releases/download/less-v608/less.exe -o less.exe
    # the below was renamed because it was failing to download for darren. it should work but it wasn't
    # todo: maybe we should get rid of this aria2c dependency and just use http get?
    #aria2c https://raw.githubusercontent.com/jftuga/less-Windows/master/LICENSE -o LICENSE-for-less.txt
    aria2c https://github.com/jftuga/less-Windows/blob/master/LICENSE -o LICENSE-for-less.txt

    # Create Windows msi release package
    if (get-env _EXTRA_) == 'msi' {

        let wixRelease = $'($src)/target/wix/($releaseStem).msi'
        print $'(char nl)Start creating Windows msi package with the following contents...'
        cd $src; hr-line
        # Wix need the binaries be stored in target/release/
        cp -r ($'($dist)/*' | into glob) target/release/
        ls target/release/* | print
        cargo install cargo-wix --version 0.3.8
        cargo wix --no-build --nocapture --package nu --output $wixRelease
        # Workaround for https://github.com/softprops/action-gh-release/issues/280
        let archive = ($wixRelease | str replace --all '\' '/')
        print $'archive: ---> ($archive)';
        echo $"archive=($archive)" | save --append $env.GITHUB_OUTPUT

    } else {

        print $'(char nl)(ansi g)Archive contents:(ansi reset)'; hr-line; ls | print
        let archive = $'($dist)/($releaseStem).zip'
        7z a $archive ...(glob *)
        let pkg = (ls -f $archive | get name)
        if not ($pkg | is-empty) {
            # Workaround for https://github.com/softprops/action-gh-release/issues/280
            let archive = ($pkg | get 0 | str replace --all '\' '/')
            print $'archive: ---> ($archive)'
            echo $"archive=($archive)" | save --append $env.GITHUB_OUTPUT
        }
    }
}

def 'cargo-build-nu' [] {
    if $os == 'windows-latest' {
        cargo build --release --all --target $target
    } else {
        cargo build --release --all --target $target --features=static-link-openssl
    }
}

# Print a horizontal line marker
def 'hr-line' [
    --blank-line(-b)
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
