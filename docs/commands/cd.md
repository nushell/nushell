If you don't know about it alreaday, the `cd` command is very simple. It stands for 'change directory' and it does exactly that. It changes the current directory that you are in to the one specified.

Example -

```
/home/username> cd Desktop
/home/username/Desktop> now your current directory has been changed
```
Additionally, `..` takes you to the parent directory -

```
/home/username/Desktop/nested/folders> cd ..
/home/username/Desktop/nested>cd ..
/home/username/Desktop> cd ../Documents/school_related
/home/username/Documents/school_related> cd ../../..
/home/>
```

And `/` takes you to the root of the filesystem, which is `/` on Linux and MacOS, and `C:\` on Windows.


If no directory is specified, it takes you to the home directory, which is `/home/your_username` on MacOS and Linux systems and `C:\Users\Your_username` on Windows.
