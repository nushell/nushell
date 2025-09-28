use std/util null_device
use std-rfc/iter only

# Cross-platform unzipping for artifacts
module unzip {
  export def main [
    file: string, # Name of file within zip to extract
    span: record # Span for error reporting
  ]: binary -> binary {
    if (which "unzip" | is-not-empty) and (^unzip -h) has "Info-ZIP" {
      infozip $file
    } else {
      error make {
        msg: "Command not found"
        help: "Install one of the following programs: 7z, unzip, tar (Windows only)"
        label: {
          text: "unable to unzip artifact"
          span: $span
        }
      }
    }
  }

  # Write input to a file, and call closure with path to file
  def with-tmpfile [func: closure]: binary -> any {
    let file = do {|file| save -fp $file; $file } (mktemp -t)
    let out = do $func $file
    rm $file
    $out
  }

  # Use "unzip" from Info-ZIP
  def infozip [file: string] {
    # Info-ZIP's unzip can't read from stdin, so write to a tempfile first
    with-tmpfile {|tmpfile|
      ^unzip -p $tmpfile $file
    }
  }
}

use unzip

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

# Get the workflow of the most recent commit with artifacts
def get-workflow-run [commits: list<string>, span: record]: nothing -> int {
  mut latest = true
  for commit in ($commits | reverse) {
    let checks = (
      ^gh api $"/repos/nushell/nushell/commits/($commit)/check-runs"
      | from json
      | get check_runs
      | where name starts-with 'std-lib'
    )

    if ($checks | is-empty) {
      $latest = false
      continue
    }

    if not $latest {
      let short = $commit | str substring 0..6
      print $"(ansi yellow)Warning: using most recent commit with artifacts ($short), which is not the most recent on the PR(ansi reset)"
    }

    return (
      $checks
      | first
      | get html_url
      # parse workflow id from url to avoid another request
      | parse "https://github.com/nushell/nushell/actions/runs/{workflow_id}/job/51149655683"
      | only workflow_id
    )
  }

  error make {
    msg: $"No artifacts"
    label: {
      text: $"no commits matching criteria have artifacts"
      span: $span
    }
    help: "Note that artifacts are deleted after 14 days"
  }

  # BUG: Unreachable echo to appease parse-time type checking
  echo
}

# Get artifacts associated with a PR
#
# Uses the latest commit if not specified
def get-artifacts [
  number: record<item: int, span: record>
  platform: string
  span: record<start: int, end: int>
  --commit: string
] {
  # Make sure gh is available and has auth set up
  if (which ^gh | is-empty) {
    error make {
      msg: "Command not found"
      label: {
        text: "requires `gh`"
        span: $span
      }
      help: "Please install the `gh` commandline tool"
    }
  }

  try {
    ^gh auth status --hostname github.com o> $null_device
  } catch {
    error make {
      msg: "No authentication"
      label: {
        text: "requires GitHub authentication"
        span: $span
      }
      help: "Please run `gh auth login`"
    }
  }

  # Listing all artifacts requires pagination, which results in 8+ requests
  # Instead, we can do PR -> commit -> check runs -> artifacts which always is 4 requests

  # Get latest commit from PR (or use --commit)
  let commits = (
    ^gh pr view $number.item -R nushell/nushell --json commits
    | from json
    | get commits.oid
    | if $commit != null { where $it == $commit } else {}
  )

  let workflow_id = get-workflow-run $commits $number.span
  let artifacts = (
    ^gh api $"/repos/nushell/nushell/actions/runs/($workflow_id)/artifacts"
    | from json
    | get artifacts
    | into datetime created_at
    | sort-by -r created_at
    | where name == $"nu-($number.item)-($platform)"
  )

  if ($artifacts | is-empty) {
    error make {
      msg: $"No artifacts"
      label: {
        text: $"no artifacts for PR match criteria"
        span: $span
      }
      help: "Note that artifacts are deleted after 14 days"
    }
  }

  $artifacts
}

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
