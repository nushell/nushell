config n
config n foo bar -
config n foo bar l --l

# detail
def "config n foo bar" [
  f: path
  --long (-s): int # test flag
] {
  echo "🤔🐘"
  | str substring (str substring -)
}
