use api.nu *
use unzip.nu

# Download a Nushell binary from a pull request CI artifact
export def "download pr" [
  # The PR number to download the Nushell binary from
  number: int
  # Use specific commit from branch
  --commit: string
  # Which platform to download for
  --platform: string
  # For internal use only
  --head: oneof<>
]: nothing -> binary {
  let span = (metadata $head).span
  let number = { item: $number, span: (metadata $number).span }

  let platform = get-platform $span $platform
  let artifacts = get-artifacts $number $platform $span --commit=$commit | first

  ^gh api $artifacts.archive_download_url | unzip "nu" $span
}

# Run Nushell by downloading a CI artifact from a pull request
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

  let dir = $nu.temp-path | path join "nushell-run-pr"
  mkdir $dir

  let platform = get-platform $span
  let artifact = get-artifacts $number $platform $span --commit=$commit | first

  let workflow_id = $artifact.workflow_run.id
  let binfile = $dir | path join $"nu-($number.item)-($workflow_id)"

  if ($binfile | path exists) {
    print $"Using previously downloaded binary from workflow run ($workflow_id)"
  } else {
    print $"Downloading binary from workflow run ($workflow_id)..."
    ^gh api $artifact.archive_download_url
    | unzip "nu" $span
    | save -p $binfile
  }

  if $nu.os-info.family == "unix" {
    chmod +x $binfile
  }

  ^$binfile ...$rest
}

def get-platform [span: record, platform?: string] {
  match $nu.os-info.name {
    _ if $platform != null => $platform
    "linux" => "ubuntu-22.04"
    "macos" => "macos-latest"
    "windows" => "windows-latest"
    $platform => {
      error make {
        msg: "Unsupported platform",
        label: {
          text: $"($platform) not supported"
          span: $span
        }
      }
    }
  }
}
