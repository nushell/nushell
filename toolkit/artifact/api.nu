use std-rfc/iter only
use std/util null_device

# Get the workflow of the most recent commit with artifacts
export def get-workflow-run [commits: list<string>, span: record]: nothing -> int {
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
      | parse "https://github.com/nushell/nushell/actions/runs/{workflow_id}/job/{job_id}"
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
export def get-artifacts [
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
