#!/usr/bin/env nu

# Created: 2025/05/21 19:05:20
# Description:
#   A script to build Windows MSI packages for NuShell. Need wix 6.0 to be installed.
#   The script will download the specified NuShell release, extract it, and create an MSI package.
#   Can be run locally or in GitHub Actions.
# To run this script locally:
#   load-env { TARGET: 'x86_64-pc-windows-msvc' REF: '0.103.0' GITHUB_REPOSITORY: 'nushell/nushell' }
#   nu .github/workflows/release-msi.nu

def build-msi [] {
    let target = $env.TARGET
    # We should read the version from the environment variable first
    # As we may build the MSI package for a specific version not the latest one
    let version = $env.MSI_VERSION? | default (open Cargo.toml | get package.version)
    let arch = if $nu.os-info.arch =~ 'x86_64' { 'x64' } else { 'arm64' }

    print $'Building msi package for (ansi g)($target)(ansi reset) with version (ansi g)($version)(ansi reset) from tag (ansi g)($env.REF)(ansi reset)...'
    fetch-nu-pkg
    # Create extra Windows msi release package if dotnet and wix are available
    let installed = [dotnet wix] | all { (which $in | length) > 0 }
    if $installed and (wix --version | split row . | first | into int) >= 6 {

        print $'(char nl)Start creating Windows msi package with the following contents...'
        cd wix; hr-line
        cp nu/README.txt .
        ls -f nu/* | print
        ./nu/nu.exe -c $'NU_RELEASE_VERSION=($version) dotnet build -c Release -p:Platform=($arch)'
        glob **/*.msi | print
        # Workaround for https://github.com/softprops/action-gh-release/issues/280
        let wixRelease = (glob **/*.msi | where $it =~ bin | get 0 | str replace --all '\' '/')
        let msi = $'($wixRelease | path dirname)/nu-($version)-($target).msi'
        mv $wixRelease $msi
        print $'MSI archive: ---> ($msi)';
        # Run only in GitHub Actions
        if ($env.GITHUB_ACTIONS? | default false | into bool) {
            echo $"msi=($msi)(char nl)" o>> $env.GITHUB_OUTPUT
        }
    }
}

def fetch-nu-pkg [] {
    mkdir wix/nu
    # See: https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/store-information-in-variables#default-environment-variables
    gh release download $env.REF --repo $env.GITHUB_REPOSITORY --pattern $'*-($env.TARGET).zip' --dir wix/nu
    cd wix/nu
    let pkg = ls *.zip | get name.0
    unzip $pkg
    rm $pkg
    ls | print
}

# Print a horizontal line marker
def 'hr-line' [
    --blank-line(-b)
] {
    print $'(ansi g)---------------------------------------------------------------------------->(ansi reset)'
    if $blank_line { char nl }
}

alias main = build-msi
