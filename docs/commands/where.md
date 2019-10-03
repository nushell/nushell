# where

This command filters the content of a table based on a condition passed as a parameter, which must be a boolean expression making use of any of the table columns. Other commands such as `ls` are capable of feeding `where` with their output through pipelines.

## Usage
```shell
> [input-command] | where [condition]
```

## Examples 

```shell
> ls | where size > 4kb
----+----------------+------+----------+----------+----------------+----------------
 #  | name           | type | readonly | size     | accessed       | modified 
----+----------------+------+----------+----------+----------------+----------------
 0  | IMG_1291.jpg   | File |          | 115.5 KB | a month ago    | 4 months ago 
 1  | README.md      | File |          | 11.1 KB  | 2 days ago     | 2 days ago 
 2  | IMG_1291.png   | File |          | 589.0 KB | a month ago    | a month ago 
 3  | IMG_1381.jpg   | File |          | 81.0 KB  | a month ago    | 4 months ago 
 4  | butterfly.jpeg | File |          | 4.2 KB   | a month ago    | a month ago 
 5  | Cargo.lock     | File |          | 199.6 KB | 22 minutes ago | 22 minutes ago
```

```shell
> ps | where cpu > 10
---+-------+----------+-------+-----------------------------
 # | pid   | status   | cpu   | name 
---+-------+----------+-------+-----------------------------
 0 | 1992  | Sleeping | 44.52 | /usr/bin/gnome-shell 
 1 | 1069  | Sleeping | 16.15 |  
 2 | 24116 | Sleeping | 13.70 | /opt/google/chrome/chrome 
 3 | 21976 | Sleeping | 12.67 | /usr/share/discord/Discord
```
