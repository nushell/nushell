# Download a Nushell binary from a CI artifact from a pull request
#
# Requires the `gh` command line tool with authentication set up
export def download-pr [
  number: int,
  --commit: string, # Use specific commit from branch
  --platform: string, # Which platform to download for
]: nothing -> binary {
    let platform = match $nu.os-info.name {
        _ if $platform != null => $platform,
        "linux" => "ubuntu-22.04",
        "macos" => "macos-latest"
        "windows" => "windows-latest"
        _ => { error make { msg: "Your platform isn't supported, please use the --platform argument" } }
    }

    let url = (
        gh api /repos/nushell/nushell/actions/artifacts
        | from json
        | get artifacts
        | into datetime created_at
        | sort-by -r created_at
        | where name == $"nu-($number)-($platform)"
    )

    # TODO error checking
    | first
    | get archive_download_url

    # TODO unzip from stdin
    # TODO crossplat unzip
    let tmp = (mktemp -t -d)
    let zip = $tmp | path join artifact.zip
    gh api $url

}

# Run Nushell by downloading a CI artifact from a pull request
export def --wrapped run-pr [
  number: int, # The PR number to download the Nushell binary from
  ...$rest     # Arguments to pass to Nushell
]: nothing -> nothing {

    unzip -d $tmp $zip
    ^($tmp | path join nu)
}
