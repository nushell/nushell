# exit

Exits the nu shell. If you have multiple nu shells, use `exit --now` to exit all of them.

## Examples 

```shell
> exit
```

```
/home/username/stuff/books> shells
---+---+------------+----------------------------
 # |   | name       | path 
---+---+------------+----------------------------
 0 |   | filesystem | /home/username/stuff/notes 
 1 |   | filesystem | /home/username/stuff/videos 
 2 | X | filesystem | /home/username/stuff/books 
---+---+------------+----------------------------
/home/username/stuff/books> exit
/home/username/stuff/videos> shells
---+---+------------+----------------------------
 # |   | name       | path 
---+---+------------+----------------------------
 0 |   | filesystem | /home/username/stuff/notes 
 1 | X | filesystem | /home/username/stuff/videos 
---+---+------------+----------------------------
/home/username/stuff/videos> exit --now
exits both the shells
```
