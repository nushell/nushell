# group-by

This command creates a new table with the data from the table rows grouped by the column given.

## Examples

Let's say we have this table of all countries in the world sorted by their population:

```shell
> open countries_by_population.json | from-json | first 10
━━━┯━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━
 # │ rank │ country or area │ UN continental region │ UN statistical region │ population 2018 │ population 2019 │ change
───┼──────┼─────────────────┼───────────────────────┼───────────────────────┼─────────────────┼─────────────────┼────────
 0 │ 1    │ China           │ Asia                  │ Eastern Asia          │ 1,427,647,786   │ 1,433,783,686   │ +0.4%
 1 │ 2    │ India           │ Asia                  │ Southern Asia         │ 1,352,642,280   │ 1,366,417,754   │ +1.0%
 2 │ 3    │ United States   │ Americas              │ Northern America      │ 327,096,265     │ 329,064,917     │ +0.6%
 3 │ 4    │ Indonesia       │ Asia                  │ South-eastern Asia    │ 267,670,543     │ 270,625,568     │ +1.1%
 4 │ 5    │ Pakistan        │ Asia                  │ Southern Asia         │ 212,228,286     │ 216,565,318     │ +2.0%
 5 │ 6    │ Brazil          │ Americas              │ South America         │ 209,469,323     │ 211,049,527     │ +0.8%
 6 │ 7    │ Nigeria         │ Africa                │ Western Africa        │ 195,874,683     │ 200,963,599     │ +2.6%
 7 │ 8    │ Bangladesh      │ Asia                  │ Southern Asia         │ 161,376,708     │ 163,046,161     │ +1.0%
 8 │ 9    │ Russia          │ Europe                │ Eastern Europe        │ 145,734,038     │ 145,872,256     │ +0.1%
 9 │ 10   │ Mexico          │ Americas              │ Central America       │ 126,190,788     │ 127,575,529     │ +1.1%
━━━┷━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━
```

Here we have listed only the first 10 lines. In total this table has got 233 rows which is to big to get information easily out of it.

We can use the `group-by` command on 'UN statistical region' to create a table per continental region.

```shell
> open countries_by_population.json | from-json | group-by "UN continental region"
━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━
 Asia             │ Americas         │ Africa           │ Europe           │ Oceania
──────────────────┼──────────────────┼──────────────────┼──────────────────┼──────────────────
 [table: 51 rows] │ [table: 53 rows] │ [table: 58 rows] │ [table: 48 rows] │ [table: 23 rows]
━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━
```

Now we can already get some informations like "which continental regions are there" and "how many countries are in each region". 
If we want to see only the countries in the continental region of Oceania we can type:

```shell
> open countries_by_population.json | from-json | group-by "UN continental region" | get Oceania
━━━━┯━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━┯━━━━━━━━
 #  │ rank │ country or area                │ UN continental region │ UN statistical region     │ population 2018 │ population 2019 │ change
────┼──────┼────────────────────────────────┼───────────────────────┼───────────────────────────┼─────────────────┼─────────────────┼────────
  0 │ 55   │ Australia                      │ Oceania               │ Australia and New Zealand │ 24,898,152      │ 25,203,198      │ +1.2%
  1 │ 98   │ Papua New Guinea               │ Oceania               │ Melanesia                 │ 8,606,323       │ 8,776,109       │ +2.0%
  2 │ 125  │ New Zealand                    │ Oceania               │ Australia and New Zealand │ 4,743,131       │ 4,783,063       │ +0.8%
  3 │ 161  │ Fiji                           │ Oceania               │ Melanesia                 │ 883,483         │ 889,953         │ +0.7%
  4 │ 166  │ Solomon Islands                │ Oceania               │ Melanesia                 │ 652,857         │ 669,823         │ +2.6%
  5 │ 181  │ Vanuatu                        │ Oceania               │ Melanesia                 │ 292,680         │ 299,882         │ +2.5%
  6 │ 183  │ New Caledonia                  │ Oceania               │ Melanesia                 │ 279,993         │ 282,750         │ +1.0%
  7 │ 185  │ French Polynesia               │ Oceania               │ Polynesia                 │ 277,679         │ 279,287         │ +0.6%
  8 │ 188  │ Samoa                          │ Oceania               │ Polynesia                 │ 196,129         │ 197,097         │ +0.5%
  9 │ 191  │ Guam                           │ Oceania               │ Micronesia                │ 165,768         │ 167,294         │ +0.9%
 10 │ 193  │ Kiribati                       │ Oceania               │ Micronesia                │ 115,847         │ 117,606         │ +1.5%
 11 │ 194  │ Federated States of Micronesia │ Oceania               │ Micronesia                │ 112,640         │ 113,815         │ +1.0%
 12 │ 196  │ Tonga                          │ Oceania               │ Polynesia                 │ 110,589         │ 110,940         │ +0.3%
 13 │ 207  │ Marshall Islands               │ Oceania               │ Micronesia                │ 58,413          │ 58,791          │ +0.6%
 14 │ 209  │ Northern Mariana Islands       │ Oceania               │ Micronesia                │ 56,882          │ 56,188          │ −1.2%
 15 │ 210  │ American Samoa                 │ Oceania               │ Polynesia                 │ 55,465          │ 55,312          │ −0.3%
 16 │ 221  │ Palau                          │ Oceania               │ Micronesia                │ 17,907          │ 18,008          │ +0.6%
 17 │ 222  │ Cook Islands                   │ Oceania               │ Polynesia                 │ 17,518          │ 17,548          │ +0.2%
 18 │ 224  │ Tuvalu                         │ Oceania               │ Polynesia                 │ 11,508          │ 11,646          │ +1.2%
 19 │ 225  │ Wallis and Futuna              │ Oceania               │ Polynesia                 │ 11,661          │ 11,432          │ −2.0%
 20 │ 226  │ Nauru                          │ Oceania               │ Micronesia                │ 10,670          │ 10,756          │ +0.8%
 21 │ 231  │ Niue                           │ Oceania               │ Polynesia                 │ 1,620           │ 1,615           │ −0.3%
 22 │ 232  │ Tokelau                        │ Oceania               │ Polynesia                 │ 1,319           │ 1,340           │ +1.6%
━━━━┷━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━┷━━━━━━━━
```
