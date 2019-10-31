# open

Loads a file into a cell, convert it to table if possible (avoid by appending `--raw` flag)

## Example

```shell
> cat user.yaml
- Name: Peter
  Age: 30
  Telephone: 88204828
  Country: Singapore
- Name: Michael
  Age: 42
  Telephone: 44002010
  Country: Spain
- Name: Will
  Age: 50
  Telephone: 99521080
  Country: Germany
> open user.yaml
━━━┯━━━━━━━━━┯━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━
 # │ Name    │ Age │ Telephone │ Country
───┼─────────┼─────┼───────────┼───────────
 0 │ Peter   │  30 │  88204828 │ Singapore
 1 │ Michael │  42 │  44002010 │ Spain
 2 │ Will    │  50 │  99521080 │ Germany
━━━┷━━━━━━━━━┷━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━
> open user.yaml --raw
- Name: Peter
  Age: 30
  Telephone: 88204828
  Country: Singapore
- Name: Michael
  Age: 42
  Telephone: 44002010
  Country: Spain
- Name: Will
  Age: 50
  Telephone: 99521080
  Country: Germany
```

```shell
> cat user.json
[
	{
		"Name": "Peter",
		"Age": 30,
		"Telephone": 88204828,
		"Country": "Singapore"
	},
	{
		"Name": "Michael",
		"Age": 42,
		"Telephone": 44002010,
		"Country": "Spain"
	},
	{
		"Name": "Will",
		"Age": 50,
		"Telephone": 99521080,
		"Country": "Germany"
	}
]
> open user.json
━━━┯━━━━━━━━━┯━━━━━┯━━━━━━━━━━━┯━━━━━━━━━━━
 # │ Name    │ Age │ Telephone │ Country
───┼─────────┼─────┼───────────┼───────────
 0 │ Peter   │  30 │  88204828 │ Singapore
 1 │ Michael │  42 │  44002010 │ Spain
 2 │ Will    │  50 │  99521080 │ Germany
━━━┷━━━━━━━━━┷━━━━━┷━━━━━━━━━━━┷━━━━━━━━━━━
> open user.json --raw
[
	{
		"Name": "Peter",
		"Age": 30,
		"Telephone": 88204828,
		"Country": "Singapore"
	},
	{
		"Name": "Michael",
		"Age": 42,
		"Telephone": 44002010,
		"Country": "Spain"
	},
	{
		"Name": "Will",
		"Age": 50,
		"Telephone": 99521080,
		"Country": "Germany"
	}
]
```