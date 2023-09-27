use std assert


# This is the custom command 1 for is-admin:

#[test]
def is-admin_return_iamroot_if_nushell_is_running_with_adminroot_privileges_and_iamnotroot_if_not_1 [] {
  let result = (if (is-admin) { "iamroot" } else { "iamnotroot" })
  assert ($result == iamnotroot)
}


