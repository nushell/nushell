use ../nu-utils/standard_library/std.nu *

export def test_output [] {
    register nu_plugin_py-quickstart.py
    assert equal (py-quickstart) 0
    assert equal (123 | py-quickstart) 0
    assert equal ("magic" | py-quickstart) 42
}

export def test_zzz_unregister_warning [] {
    log warning "Do not forget to remove the py-quickstart plugin manually. (The is no `unregister` command yet.)"
}
