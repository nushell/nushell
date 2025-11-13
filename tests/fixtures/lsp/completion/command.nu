config n
config n foo bar -
config n foo bar c --l

# detail
def "config n foo bar" [
  f: path
  --long (-s): int # test flag
] {
  echo "🤔🐘"
  | str substring (str substring -)
}

config n # don't panic!

'1' | into int -e big --endian big

# command-wide completion
def "nu-complete foo" [spans: list] { [($spans | last)] }

@complete "nu-complete foo"
def --wrapped "foo" [...rest] { }

foo bar baz

@complete external
def --wrapped bar [...rest] { }

bar baz  qux
