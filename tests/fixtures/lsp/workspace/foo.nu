export def foooo [
  --param(-p): int
] {
  $param
}

export  def "foo str" [] { "foo" }

export module "mod name" {
  # cmt
  export module "sub module" {
    export def "cmd name" [] { }
  }
}

export module cst_mod {
  export module "sub module" {
    export const var_name = "const value"
  }
}
