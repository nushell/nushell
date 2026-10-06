export use api.nu *
use unzip.nu

# these are the currently provided platforms in our ci
const platforms = ["linux-x86_64", "macos-aarch64", "windows-x86_64"]
const this_platform = $"($nu.os-info.name)-($nu.os-info.arch)"

# Download a Nushell binary from a pull request CI artifact.
@category "toolkit"
@search-terms download pr artifact binary ci gh
@example "Download the binary from PR #1234" { toolkit download pr 1234 }
@example "Download for a specific platform" { toolkit download pr 1234 --platform macos-aarch64 }
export def "download pr" [
  # The PR number to download the Nushell binary from
  number: int
  # Use specific commit from branch
  --commit: string
  # OS and architecture to download for (defaults to the current platform)
  --platform: string@$platforms = $this_platform
  # For internal use only
  --head: oneof<>
]: nothing -> binary {
  let span = (metadata $head).span
  let number = { item: $number, span: (metadata $number).span }

  let artifacts = get-artifacts $number $platform $span --commit=$commit | first
  let filename = if $platform starts-with "windows-" { "nu.exe" } else { "nu" }

  ^gh api $artifacts.archive_download_url | unzip $filename $span
}

# Run Nushell by downloading a CI artifact from a pull request.
@category "toolkit"
@search-terms run pr artifact binary ci gh try
@example "Run the binary from PR #1234" { toolkit run pr 1234 }
@example "Run a specific commit and pass arguments" { toolkit run pr 1234 --commit abcdef -- --version }
export def --wrapped "run pr" [
  # The PR number to download the Nushell binary from
  number: int
  # Use specific commit from branch
  --commit: string
  # Arguments to pass to Nushell
  ...$rest
  # For internal use only
  --head: oneof<>
]: nothing -> nothing {
  let span = (metadata $head).span
  let number = { item: $number, span: (metadata $number).span }

  let dir = $nu.temp-dir | path join "nushell-run-pr"
  mkdir $dir

  let platform = $"($nu.os-info.name)-($nu.os-info.arch)"
  let artifact = get-artifacts $number $platform $span --commit=$commit | first

  let workflow_id = $artifact.workflow_run.id
  let extension = if $nu.os-info.name == "windows" { ".exe" } else { "" }
  let filename = $"nu($extension)"
  let binfile = $dir | path join $"nu-($number.item)-($workflow_id)-($platform)($extension)"

  if ($binfile | path exists) {
    print $"Using previously downloaded binary from workflow run ($workflow_id)"
  } else {
    print $"Downloading binary from workflow run ($workflow_id)..."
    ^gh api $artifact.archive_download_url
    | unzip $filename $span
    | save -p $binfile
  }

  if $nu.os-info.family == "unix" {
    chmod +x $binfile
  }

  ^$binfile ...$rest
}
