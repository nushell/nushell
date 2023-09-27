use std assert


# This is the custom command 1 for overlay_list:

#[test]
def overlay_list_get_the_last_activated_overlay_1 [] {
  let result = (module spam { export def foo [] { "foo" } }
    overlay use spam
    overlay list | last)
  assert ($result == spam)
}


