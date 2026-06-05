export def foooo [
  --param(-p): int
] {
  $param
}

export  def "foo str" [] { "foo" }

export module "mod name" {
  export module "sub module" {
    export def "cmd name long" [] { }
  }; export use "sub module"
}; export use "mod name"

# cmt
export module cst_mod {
  # sub cmt
  export module "sub module" {
    # sub sub cmt
    export module "sub sub module" {
      export const var_name = "const value"
    }; export use "sub sub module"
  }; export use "sub module"

  @category foo
  export  def module_attribute_foo [] { }

  @complete external
  export extern module_attribute_baz []
}; export use cst_mod

@category foo
export def block_attribute_foo [] { }

@complete external
export extern block_attribute_baz []
