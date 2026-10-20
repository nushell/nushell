module m {
  export def f [] { 2 }
  export const c = 1
  export-env { $env.A = 1 }
  export alias g = f
  export module inner { }
  export extern e []
}
