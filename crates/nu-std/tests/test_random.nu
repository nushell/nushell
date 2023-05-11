use std *

export def test_list_length [] {
    assert equal (random-list bool 3) 3
    assert equal (random-list chars 3) 3
    assert equal (random-list decimal 3) 3
    assert equal (random-list dice 3) 3
    assert equal (random-list integer 3) 3
    assert equal (random-list uuid 3) 3
}
