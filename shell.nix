(import (fetchTarball "https://github.com/edolstra/flake-compat/archive/master.tar.gz") {
  src = builtins.fetchGit ./.;
})
.shellNix
