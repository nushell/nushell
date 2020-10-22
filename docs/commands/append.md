# append

Append a row to the table.

## Examples

Given the following text file `cities.txt` containing cities:

```shell
Canberra
London
Nairobi
Washington
```

And getting back a Nu table:

```shell
> open cities.txt | lines
───┬────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
───┴────────────
```

Add the city named `Beijing` like so:

```shell
> open cities.txt | lines | append Beijing
───┬────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
 4 │ Beijing
───┴────────────
```

It's not possible to add multiple rows at once, so you'll need to use `append` multiple times:

```shell
> open cities.txt | lines | append Beijing | append "Buenos Aires"
───┬──────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
 4 │ Beijing
 5 │ Buenos Aires
───┴──────────────
```

So far we have been working with a table without a column, which leaves us with plain rows. Let's `wrap` the plain rows into a column called `city` and save it as a json file called `cities.json`:

Before we save, let's check how it looks after wrapping:

```shell
open cities.txt | lines | wrap city
───┬────────────
 # │ city
───┼────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
───┴────────────
```

And save:

`> open cities.txt | lines | wrap city | save cities.json`

Since we will be working with rows that have a column, appending like before won't quite give us back what we want:

```shell
> open cities.json | append Guayaquil
───┬────────────
 # │ city
───┼────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
───┴────────────
───┬───────────
 4 │ Guayaquil
───┴───────────
```

We append a row literal directly:

```shell
> open cities.json | append [[city]; [Guayaquil]]
───┬────────────
 # │ city
───┼────────────
 0 │ Canberra
 1 │ London
 2 │ Nairobi
 3 │ Washington
 4 │ Guayaquil
───┴────────────
```