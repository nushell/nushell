# Cross-platform unzipping for artifacts
export def main [
  filename: string, # Name of file within zip to extract
  span: record # Span for error reporting
]: binary -> binary {
  # Store zip file to temporary file
  let zipfile = do {|file| save -fp $file; $file } (mktemp -t)

  let programs = [
    [preconditions, closure];
    [(which "gzip" | is-not-empty), { gzip $zipfile }]
    [((which "tar" | is-not-empty) and $nu.os-info.name == "windows"), { tar $zipfile $filename }]
    [(which "7z" | is-not-empty), { 7z $zipfile $filename }]
    [(which "unzip" | is-not-empty), { unzip $zipfile $filename }]
  ]

  # Attempt available programs
  for program in $programs {
    if not $program.preconditions {
      continue
    }

    try {
      let out = do $program.closure
      rm $zipfile
      return $out
    }
  }

  error make {
    msg: "Command not found"
    help: "Install one of the following programs: gzip, 7z, unzip, tar (Windows only)"
    label: {
      text: "failed to unzip artifact"
      span: $span
    }
  }

  # BUG: Unreachable echo to appease parse-time type checking
  echo
}

# tar can unzip files on Windows
def tar [zipfile: string, filename: string] {
  ^tar -Oxf $zipfile $filename
}

# Some versions of gzip can extract single files from zip files
def gzip [zipfile: string] {
  open -r $zipfile | ^gzip -d
}

# Use 7zip
def 7z [zipfile: string, filename: string] {
  ^7z x $zipfile -so $filename
}

# Use unzip tool (Info-ZIP, macOS, BSD)
def unzip [zipfile: string, filename: string] {
  ^unzip -p $zipfile $filename
}
