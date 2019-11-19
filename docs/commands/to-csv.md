# to-csv

Converts table data into csv text.

## Example

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────
 0 │ X │ filesystem │ /home/shaurya
 1 │   │ filesystem │ /home/shaurya/Pictures
 2 │   │ filesystem │ /home/shaurya/Desktop
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━
> shells | to-csv
 ,name,path
X,filesystem,/home/shaurya
 ,filesystem,/home/shaurya/Pictures
 ,filesystem,/home/shaurya/Desktop
```

```shell
> open caco3_plastics.csv
━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━━━━┯━━━━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━━━━
 # │ importer     │ shipper      │ tariff_item │ name         │ origin   │ shipped_at │ arrived_at │ net_weight │ fob_price │ cif_price │ cif_per_net_
   │              │              │             │              │          │            │            │            │           │           │ weight
───┼──────────────┼──────────────┼─────────────┼──────────────┼──────────┼────────────┼────────────┼────────────┼───────────┼───────────┼──────────────
 0 │ PLASTICOS    │ S A REVERTE  │ 2509000000  │ CARBONATO DE │ SPAIN    │ 18/03/2016 │ 17/04/2016 │ 81,000.00  │ 14,417.58 │ 18,252.34 │ 0.23
   │ RIVAL CIA    │              │             │ CALCIO TIPO  │          │            │            │            │           │           │
   │ LTDA         │              │             │ CALCIPORE    │          │            │            │            │           │           │
   │              │              │             │ 160 T AL     │          │            │            │            │           │           │
 1 │ MEXICHEM     │ OMYA ANDINA  │ 2836500000  │ CARBONATO    │ COLOMBIA │ 07/07/2016 │ 10/07/2016 │ 26,000.00  │ 7,072.00  │ 8,127.18  │ 0.31
   │ ECUADOR S.A. │ S A          │             │              │          │            │            │            │           │           │
 2 │ PLASTIAZUAY  │ SA REVERTE   │ 2836500000  │ CARBONATO DE │ SPAIN    │ 27/07/2016 │ 09/08/2016 │ 81,000.00  │ 8,100.00  │ 11,474.55 │ 0.14
   │ SA           │              │             │ CALCIO       │          │            │            │            │           │           │
 3 │ PLASTICOS    │ AND          │ 2836500000  │ CALCIUM      │ TURKEY   │ 04/10/2016 │ 11/11/2016 │ 100,000.00 │ 17,500.00 │ 22,533.75 │ 0.23
   │ RIVAL CIA    │ ENDUSTRIYEL  │             │ CARBONATE    │          │            │            │            │           │           │
   │ LTDA         │ HAMMADDELER  │             │ ANADOLU      │          │            │            │            │           │           │
   │              │ DIS TCARET   │             │ ANDCARB CT-1 │          │            │            │            │           │           │
   │              │ LTD.STI.     │             │              │          │            │            │            │           │           │
 4 │ QUIMICA      │ SA REVERTE   │ 2836500000  │ CARBONATO DE │ SPAIN    │ 24/06/2016 │ 12/07/2016 │ 27,000.00  │ 3,258.90  │ 5,585.00  │ 0.21
   │ COMERCIAL    │              │             │ CALCIO       │          │            │            │            │           │           │
   │ QUIMICIAL    │              │             │              │          │            │            │            │           │           │
   │ CIA. LTDA.   │              │             │              │          │            │            │            │           │           │
 5 │ PICA         │ OMYA ANDINA  │ 3824909999  │ CARBONATO DE │ COLOMBIA │ 01/01/1900 │ 18/01/2016 │ 66,500.00  │ 12,635.00 │ 18,670.52 │ 0.28
   │ PLASTICOS    │ S.A          │             │ CALCIO       │          │            │            │            │           │           │
   │ INDUSTRIALES │              │             │              │          │            │            │            │           │           │
   │ C.A.         │              │             │              │          │            │            │            │           │           │
 6 │ PLASTIQUIM   │ OMYA ANDINA  │ 3824909999  │ CARBONATO DE │ COLOMBIA │ 01/01/1900 │ 25/10/2016 │ 33,000.00  │ 6,270.00  │ 9,999.00  │ 0.30
   │ S.A.         │ S.A NIT      │             │ CALCIO       │          │            │            │            │           │           │
   │              │ 830.027.386- │             │ RECUBIERTO   │          │            │            │            │           │           │
   │              │ 6            │             │ CON ACIDO    │          │            │            │            │           │           │
   │              │              │             │ ESTEARICO    │          │            │            │            │           │           │
   │              │              │             │ OMYA CARB 1T │          │            │            │            │           │           │
   │              │              │             │ CG BBS 1000  │          │            │            │            │           │           │
 7 │ QUIMICOS     │ SIBELCO      │ 3824909999  │ CARBONATO DE │ COLOMBIA │ 01/11/2016 │ 03/11/2016 │ 52,000.00  │ 8,944.00  │ 13,039.05 │ 0.25
   │ ANDINOS      │ COLOMBIA SAS │             │ CALCIO       │          │            │            │            │           │           │
   │ QUIMANDI     │              │             │ RECUBIERTO   │          │            │            │            │           │           │
   │ S.A.         │              │             │              │          │            │            │            │           │           │
 8 │ TIGRE        │ OMYA ANDINA  │ 3824909999  │ CARBONATO DE │ COLOMBIA │ 01/01/1900 │ 28/10/2016 │ 66,000.00  │ 11,748.00 │ 18,216.00 │ 0.28
   │ ECUADOR S.A. │ S.A NIT      │             │ CALCIO       │          │            │            │            │           │           │
   │ ECUATIGRE    │ 830.027.386- │             │ RECUBIERTO   │          │            │            │            │           │           │
   │              │ 6            │             │ CON ACIDO    │          │            │            │            │           │           │
   │              │              │             │ ESTEARICO    │          │            │            │            │           │           │
   │              │              │             │ OMYACARB 1T  │          │            │            │            │           │           │
   │              │              │             │ CG BPA 25 NO │          │            │            │            │           │           │
━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━━━━┷━━━━━━━━━━━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━━━━
> open caco3_plastics.csv | to-csv
importer,shipper,tariff_item,name,origin,shipped_at,arrived_at,net_weight,fob_price,cif_price,cif_per_net_weight
PLASTICOS RIVAL CIA LTDA,S A REVERTE,2509000000,CARBONATO DE CALCIO TIPO CALCIPORE 160 T AL,SPAIN,18/03/2016,17/04/2016,"81,000.00","14,417.58","18,252.34",0.23
MEXICHEM ECUADOR S.A.,OMYA ANDINA S A,2836500000,CARBONATO,COLOMBIA,07/07/2016,10/07/2016,"26,000.00","7,072.00","8,127.18",0.31
PLASTIAZUAY SA,SA REVERTE,2836500000,CARBONATO DE CALCIO,SPAIN,27/07/2016,09/08/2016,"81,000.00","8,100.00","11,474.55",0.14
PLASTICOS RIVAL CIA LTDA,AND ENDUSTRIYEL HAMMADDELER DIS TCARET LTD.STI.,2836500000,CALCIUM CARBONATE ANADOLU ANDCARB CT-1,TURKEY,04/10/2016,11/11/2016,"100,000.00","17,500.00","22,533.75",0.23
QUIMICA COMERCIAL QUIMICIAL CIA. LTDA.,SA REVERTE,2836500000,CARBONATO DE CALCIO,SPAIN,24/06/2016,12/07/2016,"27,000.00","3,258.90","5,585.00",0.21
PICA PLASTICOS INDUSTRIALES C.A.,OMYA ANDINA S.A,3824909999,CARBONATO DE CALCIO,COLOMBIA,01/01/1900,18/01/2016,"66,500.00","12,635.00","18,670.52",0.28
PLASTIQUIM S.A.,OMYA ANDINA S.A NIT 830.027.386-6,3824909999,CARBONATO DE CALCIO RECUBIERTO CON ACIDO ESTEARICO OMYA CARB 1T CG BBS 1000,COLOMBIA,01/01/1900,25/10/2016,"33,000.00","6,270.00","9,999.00",0.30
QUIMICOS ANDINOS QUIMANDI S.A.,SIBELCO COLOMBIA SAS,3824909999,CARBONATO DE CALCIO RECUBIERTO,COLOMBIA,01/11/2016,03/11/2016,"52,000.00","8,944.00","13,039.05",0.25
TIGRE ECUADOR S.A. ECUATIGRE,OMYA ANDINA S.A NIT 830.027.386-6,3824909999,CARBONATO DE  CALCIO RECUBIERTO CON ACIDO ESTEARICO OMYACARB 1T CG BPA 25 NO,COLOMBIA,01/01/1900,28/10/2016,"66,000.00","11,748.00","18,216.00",0.28
```

To use a character other than ',' to separate records, use `--separator` :

```shell
> shells
━━━┯━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━
 # │   │ name       │ path
───┼───┼────────────┼────────────────────────
 0 │ X │ filesystem │ /home/shaurya
 1 │   │ filesystem │ /home/shaurya/Pictures
 2 │   │ filesystem │ /home/shaurya/Desktop
━━━┷━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━
> shells | to-csv --separator ';'
 ;name,path
X;filesystem;/home/shaurya
 ;filesystem;/home/shaurya/Pictures
 ;filesystem;/home/shaurya/Desktop
```

The string '\t' can be used to separate on tabs. Note that this is the same as using the to-tsv command.

Newlines '\n' are not acceptable separators.

Note that separators are currently provided as strings and need to be wrapped in quotes.

It is also considered an error to use a separator greater than one char :

```shell
> open pets.txt | from-csv --separator '123'
error: Expected a single separator char from --separator
- shell:1:37
1 | open pets.txt | from-csv --separator '123'
  |                                      ^^^^^ requires a single character string input
```
