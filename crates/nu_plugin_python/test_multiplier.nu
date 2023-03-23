use ../nu-utils/standard_library/std.nu *

export def test_int [] {
    register nu_plugin_py-multiplier.py
    assert equal 3 (1 | py-multiplier 3)
    assert equal 0 (0 | py-multiplier 3)
    assert equal (-3) (-1 | py-multiplier 3)
}

export def test_bool [] {
    register nu_plugin_py-multiplier.py
    assert equal true (true | py-multiplier 3)
    assert equal false (false | py-multiplier 3)
}

export def test_nothing [] {
    register nu_plugin_py-multiplier.py
    assert equal null (null | py-multiplier 3)
}

export def test_float [] {
    register nu_plugin_py-multiplier.py
    assert equal 0.0 (0.0 | py-multiplier 3)
    assert equal 369.75 (123.25 | py-multiplier 3)
    assert equal (-2962.962) (-987.654 | py-multiplier 3)
}

export def test_string [] {
    register nu_plugin_py-multiplier.py
    assert equal "abcdabcdabcd" ("abcd" | py-multiplier 3)
}

export def test_filesize [] {
    register nu_plugin_py-multiplier.py
    assert equal 36Gb (12Gb | py-multiplier 3)
}

export def test_date [] {
    register nu_plugin_py-multiplier.py
    assert equal 2033-03-26T12:44:34 (2033-03-26T12:44:34 | py-multiplier 3)
}

export def test_list [] {
    register nu_plugin_py-multiplier.py
    assert equal [3, true, 12.3] ([1, true, 4.1] | py-multiplier 3)
}

export def test_record [] {
    register nu_plugin_py-multiplier.py
    assert equal {a:bbb c:ddd} ({a:b c:d} | py-multiplier 3)
}

export def test_closure [] {
    register nu_plugin_py-multiplier.py
    let $closure = {|e| e}
    assert equal $closure ($closure | py-multiplier 3)
}

export def test_range [] {
    register nu_plugin_py-multiplier.py
    assert equal 0..6 (0..2 | py-multiplier 3)
}

export def test_binary [] {
    register nu_plugin_py-multiplier.py
    assert equal 0x[255 127 255 127 255 127] (0x[255 127] | py-multiplier 3)
}

export def test_block [] {
    log warning "Testing Block type is not implemented yet"
}

export def test_make_error [] {
    register nu_plugin_py-multiplier.py
    assert error { || ("Please make an error." | py-multiplier 3) }
}

export def test_handle_error [] {
    log warning "Testing Error type is not implemented yet"
}

export def test_cellpath [] {
    log warning "Testing CellPath type is not implemented yet"
}

export def test_custom_value [] {
    log warning "Testing CustomValue type is not implemented yet"
}

export def test_lazy_record [] {
    log warning "Testing LazyRecord type is not implemented yet"
}

export def test_zzz_unregister_warning [] {
    log warning "Do not forget to remove the py-multiply plugin manually. (The is no `unregister` command yet.)"
}
