# Build all Windows archives and MSIs for release manually
#
# This builds std and full distributions for both aarch64 and x86_64.
#
# You need to have the cross-compilers for MSVC installed (see Visual Studio).
# If compiling on x86_64, you need ARM64 compilers and libs too, and vice versa.
export def 'release-pkg windows' [
    --artifacts-dir="artifacts" # Where to copy the final msi and zip files to
] {
    $env.RUSTFLAGS = ""
    $env.CARGO_TARGET_DIR = ""
    hide-env RUSTFLAGS
    hide-env CARGO_TARGET_DIR
    $env.OS = "windows-latest"
    $env.GITHUB_WORKSPACE = ("." | path expand)
    $env.GITHUB_OUTPUT = ("./output/out.txt" | path expand)
    let version = (open Cargo.toml | get package.version)
    mkdir $artifacts_dir
    for target in ["aarch64" "x86_64"] {
        $env.TARGET = $target ++ "-pc-windows-msvc"

        rm -rf output
        _EXTRA_=bin nu .github/workflows/release-pkg.nu
        cp $"output/nu-($version)-($target)-pc-windows-msvc.zip" $artifacts_dir

        rm -rf output
        _EXTRA_=msi nu .github/workflows/release-pkg.nu
        cp $"target/wix/nu-($version)-($target)-pc-windows-msvc.msi" $artifacts_dir
    }
}
