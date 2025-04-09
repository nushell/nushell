export def foooo [
  --param(-p): int
] {
  $param
}

export  def "foo str" [] { "foo" }

export module "mod name" {
  export module "sub module" {
    export def "cmd name" [] { }
  }
}

# cmt
export module cst_mod {
  # sub cmt
  export module "sub module" {
    export const var_name = "const value"
  }
}
