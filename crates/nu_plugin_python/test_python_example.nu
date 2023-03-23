use ../nu-utils/standard_library/std.nu *

export def test_output [] {
    register nu_plugin_python_example.py
    assert length (nu-python 12 abc) 10
    assert equal (nu-python 12 abc).0.one 0
    assert equal (nu-python 12 abc).4.two 4
    assert equal (nu-python 12 abc).8.three 16
}

export def test_zzz_unregister_warning [] {
    log warning "Do not forget to remove the python_example plugin manually. (The is no `unregister` command yet.)"
}
