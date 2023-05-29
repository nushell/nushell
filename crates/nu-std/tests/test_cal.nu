use std assert
use std cal

export def test_cal [] {
    (
        assert equal
            (cal -y --full-year 2010 | first)
            {
                year: 2010,
                sunday: null,
                monday: null,
                tuesday: null,
                wednesday: null,
                thursday: null,
                friday: 1,
                saturday: 2,
            }
    )

    (
        assert equal
            (cal -ym --full-year 2020 --month-names | where month == "february")
            [
                [year, month, sunday, monday, tuesday, wednesday, thursday, friday, saturday];

                [2020, "february", null, null, null, null, null, null,  1],
                [2020, "february",    2,    3,    4,    5,    6,    7,  8],
                [2020, "february",    9,   10,   11,   12,   13,   14, 15],
                [2020, "february",   16,   17,   18,   19,   20,   21, 22],
                [2020, "february",   23,   24,   25,   26,   27,   28, 29],
            ]
    )

    assert length (cal --full-year 2015 | default 0 friday | where friday == 13) 3
    assert length (cal --full-year 2020) 62

    (
        assert equal
            (cal --full-year 2020 -m --month-names --week-start monday | where month == january)
            [
                [month, monday, tuesday, wednesday, thursday, friday, saturday, sunday];

                ["january", null, null,  1,  2,  3,    4,    5],
                ["january",    6,    7,  8,  9, 10,   11,   12],
                ["january",   13,   14, 15, 16, 17,   18,   19],
                ["january",   20,   21, 22, 23, 24,   25,   26],
                ["january",   27,   28, 29, 30, 31, null, null],
            ]
    )

    assert equal (cal --full-year 1020 | get monday | first 4)  [null, 3, 10, 17]
}
