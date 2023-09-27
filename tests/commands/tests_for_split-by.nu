use std assert

# Parameter name:
# sig type   : record
# name       : splitter
# type       : positional
# shape      : any
# description: the splitter value to use


# This is the custom command 1 for split-by:

#[test]
def split-by_split_items_by_column_named_lang_1 [] {
  let result = ({
        '2019': [
          { name: 'andres', lang: 'rb', year: '2019' },
          { name: 'jt', lang: 'rs', year: '2019' }
        ],
        '2021': [
          { name: 'storm', lang: 'rs', 'year': '2021' }
        ]
    } | split-by lang)
  assert ($result == {rb: {2019: [{name: andres, lang: rb, year: 2019}]}, rs: {2019: [{name: jt, lang: rs, year: 2019}], 2021: [{name: storm, lang: rs, year: 2021}]}})
}


